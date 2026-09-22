use std::{collections::HashMap, error::Error, sync::mpsc::Sender, time::Duration};

use eframe::egui;
use futures::StreamExt;
use libp2p::{
    gossipsub,
    mdns,
    noise,
    swarm::{dial_opts::DialOpts, NetworkBehaviour, SwarmEvent},
    tcp,
    yamux,
    Multiaddr,
    PeerId,
};
use serde::{Deserialize, Serialize};
use tokio::{select, sync::mpsc::UnboundedReceiver};

const TOPICO: &str = "jogo-da-velha";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Mensagem {
    Jogada { posicao: u8 },
    NovaPartida { partida: u32 },
}

pub enum Comando {
    Publicar(Mensagem),
}

pub enum EventoRede {
    OponenteConectado { sou_x: bool },
    OponenteDesconectado,
    MensagemRecebida(Mensagem),
    Status(String),
    Erro(String),
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub async fn executar(
    mut comandos: UnboundedReceiver<Comando>,
    eventos: Sender<EventoRede>,
    ctx: egui::Context,
) -> Result<(), Box<dyn Error>> {
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let message_authenticity = gossipsub::MessageAuthenticity::Signed(key.clone());

            let config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .build()
                .expect("Configuração válida");

            let mut gossipsub = gossipsub::Behaviour::new(message_authenticity, config)
                .expect("Gossipsub criado");

            gossipsub
                .subscribe(&gossipsub::IdentTopic::new(TOPICO))
                .expect("Inscrição no tópico");

            Ok(Behaviour {
                gossipsub,
                mdns: mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?,
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let topico = gossipsub::IdentTopic::new(TOPICO);
    let mut oponente: Option<PeerId> = None;

    let enviar = |evento: EventoRede| {
        if let EventoRede::Status(texto) = &evento {
            println!("{texto}");
        }
        let _ = eventos.send(evento);
        ctx.request_repaint();
    };

    loop {
        select! {
            Some(Comando::Publicar(mensagem)) = comandos.recv() => {
                let bytes = serde_json::to_vec(&mensagem)?;

                if let Err(erro) = swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(topico.clone(), bytes)
                {
                    enviar(EventoRede::Erro(format!("Falha ao enviar jogada: {erro:?}")));
                }
            }

            evento = swarm.select_next_some() => match evento {
                SwarmEvent::NewListenAddr { address, .. } => {
                    enviar(EventoRede::Status(format!("Escutando em {address}")));
                }

                SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                    let mut enderecos: HashMap<PeerId, Vec<Multiaddr>> = HashMap::new();

                    for (peer_id, endereco) in peers {
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        enderecos.entry(peer_id).or_default().push(endereco);
                    }

                    for (peer_id, lista) in enderecos {
                        if swarm.is_connected(&peer_id) {
                            continue;
                        }

                        if *swarm.local_peer_id() < peer_id {
                            enviar(EventoRede::Status(format!("Peer encontrado, discando: {peer_id}")));

                            let opcoes = DialOpts::peer_id(peer_id).addresses(lista).build();
                            if let Err(erro) = swarm.dial(opcoes) {
                                enviar(EventoRede::Status(format!(
                                    "Falha ao discar para {peer_id}: {erro}"
                                )));
                            }
                        } else {
                            enviar(EventoRede::Status(format!(
                                "Peer encontrado, aguardando conexão: {peer_id}"
                            )));
                        }
                    }
                }

                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    enviar(EventoRede::Status(format!(
                        "Erro ao conectar com {peer_id:?}: {error}"
                    )));
                }

                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    enviar(EventoRede::Status(format!(
                        "Conectado a {peer_id}, aguardando inscrição no tópico"
                    )));
                }

                SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                    for (peer_id, _) in peers {
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                }

                SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Subscribed {
                    peer_id,
                    topic,
                })) => {
                    if topic == topico.hash() && oponente.is_none() {
                        oponente = Some(peer_id);
                        let sou_x = *swarm.local_peer_id() < peer_id;
                        enviar(EventoRede::OponenteConectado { sou_x });
                    }
                }

                SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Unsubscribed {
                    peer_id,
                    ..
                })) => {
                    if oponente == Some(peer_id) {
                        oponente = None;
                        enviar(EventoRede::OponenteDesconectado);
                    }
                }

                SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    message,
                    ..
                })) => {
                    if message.source.is_some() && message.source == oponente {
                        match serde_json::from_slice::<Mensagem>(&message.data) {
                            Ok(mensagem) => enviar(EventoRede::MensagemRecebida(mensagem)),
                            Err(_) => enviar(EventoRede::Erro("Mensagem inválida recebida.".to_string())),
                        }
                    }
                }

                SwarmEvent::ConnectionClosed { peer_id, num_established, cause, .. } => {
                    enviar(EventoRede::Status(format!(
                        "Conexão encerrada com {peer_id}: {cause:?}"
                    )));

                    if oponente == Some(peer_id) && num_established == 0 {
                        oponente = None;
                        enviar(EventoRede::OponenteDesconectado);
                    }
                }

                _ => {}
            }
        }
    }
}