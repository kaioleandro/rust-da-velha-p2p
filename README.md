# Jogo da Velha P2P

Jogo da velha (tic-tac-toe) para dois jogadores em rede local, sem servidor central. As duas instâncias se descobrem automaticamente via mDNS e trocam jogadas por gossipsub (libp2p). A interface gráfica é feita com [egui](https://github.com/emilk/egui)/eframe.

## Funcionalidades

- Descoberta automática de oponentes na rede local (mDNS) — não é preciso configurar IP/porta.
- Comunicação P2P direta entre os dois jogadores (libp2p: TCP + Noise + Yamux + Gossipsub).
- Alternância de símbolo (X/O) e de quem começa a cada nova partida.
- Placar de vitórias e derrotas exibido na interface, contabilizado durante a sessão.
- Detecção de vitória, empate e reinício de partida.

## Requisitos

- [Rust](https://www.rust-lang.org/tools/install) (edição 2024) e Cargo.
- Duas máquinas (ou duas instâncias) na mesma rede local com suporte a multicast/mDNS.

## Como executar

```bash
cargo run
```

Abra o programa em duas máquinas (ou dois processos) na mesma rede. Assim que um encontrar o outro, a partida começa automaticamente — um jogador recebe o símbolo X e o outro o símbolo O.

## Como jogar

1. Ao abrir o app, ele fica "Procurando oponente na rede local...".
2. Quando o oponente é encontrado, a partida inicia e o status mostra de quem é a vez.
3. Clique em uma casa do tabuleiro na sua vez para jogar.
4. Ao final da partida (vitória ou empate), use o botão **Nova partida** para jogar novamente — a vez inicial alterna a cada partida.
5. O placar de **Vitórias** e **Derrotas** no topo da tela acompanha o resultado das partidas da sessão atual.

## Estrutura do projeto

- [src/main.rs](src/main.rs) — interface gráfica (egui) e orquestração do estado da aplicação.
- [src/jogo.rs](src/jogo.rs) — regras do jogo da velha (tabuleiro, jogadas, verificação de vitória/empate).
- [src/rede.rs](src/rede.rs) — camada de rede P2P (libp2p: descoberta via mDNS e troca de mensagens via gossipsub).

## Tecnologias

- [eframe/egui](https://github.com/emilk/egui) — interface gráfica.
- [libp2p](https://libp2p.io/) — rede P2P (mDNS + Gossipsub sobre TCP/Noise/Yamux).
- [tokio](https://tokio.rs/) — runtime assíncrono.
- [serde](https://serde.rs/) / serde_json — serialização das mensagens trocadas entre os peers.
