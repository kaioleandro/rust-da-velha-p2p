mod jogo;
mod rede;

use std::sync::mpsc::{self, Receiver};

use eframe::egui;
use tokio::sync::mpsc::UnboundedSender;

use jogo::{Jogo, Resultado, Simbolo};
use rede::{Comando, EventoRede, Mensagem};

enum Estado {
    AguardandoOponente,
    Jogando { meu_simbolo: Simbolo },
}

struct App {
    estado: Estado,
    jogo: Jogo,
    aviso: Option<String>,
    status_rede: String,
    comandos: UnboundedSender<Comando>,
    eventos: Receiver<EventoRede>,
    vitorias: u32,
    derrotas: u32,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx_comandos, rx_comandos) = tokio::sync::mpsc::unbounded_channel();
        let (tx_eventos, rx_eventos) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();
        let ctx_erro = cc.egui_ctx.clone();
        let tx_erro = tx_eventos.clone();

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect("Runtime do tokio");
            if let Err(erro) = runtime.block_on(rede::executar(rx_comandos, tx_eventos, ctx)) {
                eprintln!("Erro na rede: {erro}");
                let _ = tx_erro.send(EventoRede::Erro(format!("Erro na rede: {erro}")));
                ctx_erro.request_repaint();
            }
        });

        App {
            estado: Estado::AguardandoOponente,
            jogo: Jogo::new(),
            aviso: None,
            status_rede: String::new(),
            comandos: tx_comandos,
            eventos: rx_eventos,
            vitorias: 0,
            derrotas: 0,
        }
    }

    fn atualizar_placar(&mut self, meu_simbolo: Simbolo) {
        if let Some(Resultado::Vitoria(vencedor)) = self.jogo.resultado {
            if vencedor == meu_simbolo {
                self.vitorias += 1;
            } else {
                self.derrotas += 1;
            }
        }
    }

    fn processar_eventos(&mut self) {
        while let Ok(evento) = self.eventos.try_recv() {
            match evento {
                EventoRede::OponenteConectado { sou_x } => {
                    let meu_simbolo = if sou_x { Simbolo::X } else { Simbolo::O };
                    self.estado = Estado::Jogando { meu_simbolo };
                    self.jogo.reiniciar();
                    self.aviso = None;
                }

                EventoRede::OponenteDesconectado => {
                    self.estado = Estado::AguardandoOponente;
                    self.jogo.reiniciar();
                    self.aviso = Some("O oponente se desconectou.".to_string());
                }

                EventoRede::MensagemRecebida(Mensagem::Jogada { posicao }) => {
                    if let Estado::Jogando { meu_simbolo } = self.estado {
                        if self.jogo.jogar(posicao as usize, meu_simbolo.oposto()) {
                            self.atualizar_placar(meu_simbolo);
                        } else {
                            self.aviso = Some(format!("Jogada inválida recebida (casa {posicao})."));
                        }
                    }
                }

                EventoRede::MensagemRecebida(Mensagem::NovaPartida { partida }) => {
                    if partida > self.jogo.partida {
                        self.jogo.nova_partida(partida);
                        self.aviso = None;
                    }
                }

                EventoRede::Status(texto) => {
                    self.status_rede = texto;
                }

                EventoRede::Erro(texto) => {
                    self.aviso = Some(texto);
                }
            }
        }
    }

    fn texto_status(&self) -> String {
        match self.estado {
            Estado::AguardandoOponente => "Procurando oponente na rede local...".to_string(),
            Estado::Jogando { meu_simbolo } => match self.jogo.resultado {
                Some(Resultado::Vitoria(vencedor)) if vencedor == meu_simbolo => {
                    "Você venceu!".to_string()
                }
                Some(Resultado::Vitoria(_)) => "Você perdeu.".to_string(),
                Some(Resultado::Empate) => "Deu velha! Empate.".to_string(),
                None if self.jogo.vazio() => {
                    let numero = self.jogo.partida + 1;
                    if self.jogo.vez == meu_simbolo {
                        format!("Partida {numero}. Você é {meu_simbolo} e joga primeiro!")
                    } else {
                        format!("Partida {numero}. Você é {meu_simbolo}, o oponente começa.")
                    }
                }
                None if self.jogo.vez == meu_simbolo => format!("Sua vez ({meu_simbolo})."),
                None => "Vez do oponente...".to_string(),
            },
        }
    }

    fn desenhar_tabuleiro(&mut self, ui: &mut egui::Ui) {
        let meu_simbolo = match self.estado {
            Estado::Jogando { meu_simbolo } => Some(meu_simbolo),
            Estado::AguardandoOponente => None,
        };
        let minha_vez = meu_simbolo == Some(self.jogo.vez) && self.jogo.resultado.is_none();

        egui::Grid::new("tabuleiro")
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                for posicao in 0..9 {
                    let texto = match self.jogo.tabuleiro[posicao] {
                        Some(simbolo) => simbolo.to_string(),
                        None => String::new(),
                    };

                    let botao = egui::Button::new(egui::RichText::new(texto).size(48.0))
                        .min_size(egui::vec2(90.0, 90.0));

                    let pode_jogar = minha_vez && self.jogo.tabuleiro[posicao].is_none();

                    if ui.add(botao).clicked() && pode_jogar {
                        if let Some(simbolo) = meu_simbolo {
                            if self.jogo.jogar(posicao, simbolo) {
                                self.atualizar_placar(simbolo);
                                let _ = self.comandos.send(Comando::Publicar(Mensagem::Jogada {
                                    posicao: posicao as u8,
                                }));
                            }
                        }
                    }

                    if posicao % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.processar_eventos();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Jogo da Velha P2P");
                ui.add_space(4.0);
                ui.label(format!(
                    "Vitórias: {} | Derrotas: {}",
                    self.vitorias, self.derrotas
                ));
                ui.add_space(8.0);
                ui.label(egui::RichText::new(self.texto_status()).size(18.0));

                if matches!(self.estado, Estado::AguardandoOponente) && !self.status_rede.is_empty() {
                    ui.label(egui::RichText::new(&self.status_rede).small().weak());
                }

                ui.add_space(12.0);

                self.desenhar_tabuleiro(ui);

                ui.add_space(12.0);

                let em_partida = matches!(self.estado, Estado::Jogando { .. });
                if em_partida && self.jogo.resultado.is_some() && ui.button("Nova partida").clicked() {
                    let proxima = self.jogo.partida + 1;
                    self.jogo.nova_partida(proxima);
                    self.aviso = None;
                    let _ = self.comandos.send(Comando::Publicar(Mensagem::NovaPartida {
                        partida: proxima,
                    }));
                }

                if let Some(aviso) = &self.aviso {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::RED, aviso);
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let opcoes = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([340.0, 480.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Jogo da Velha P2P",
        opcoes,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}