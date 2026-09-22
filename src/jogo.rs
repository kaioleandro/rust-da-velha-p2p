use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Simbolo {
    X,
    O,
}

impl Simbolo {
    pub fn oposto(self) -> Simbolo {
        match self {
            Simbolo::X => Simbolo::O,
            Simbolo::O => Simbolo::X,
        }
    }
}

impl fmt::Display for Simbolo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Simbolo::X => write!(f, "X"),
            Simbolo::O => write!(f, "O"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Resultado {
    Vitoria(Simbolo),
    Empate,
}

const LINHAS: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

pub struct Jogo {
    pub tabuleiro: [Option<Simbolo>; 9],
    pub vez: Simbolo,
    pub resultado: Option<Resultado>,
    pub partida: u32,
}

impl Jogo {
    pub fn new() -> Self {
        Jogo {
            tabuleiro: [None; 9],
            vez: Simbolo::X,
            resultado: None,
            partida: 0,
        }
    }

    pub fn reiniciar(&mut self) {
        *self = Jogo::new();
    }

    pub fn nova_partida(&mut self, partida: u32) {
        self.tabuleiro = [None; 9];
        self.resultado = None;
        self.partida = partida;
        self.vez = if partida % 2 == 0 { Simbolo::X } else { Simbolo::O };
    }

    pub fn vazio(&self) -> bool {
        self.tabuleiro.iter().all(|casa| casa.is_none())
    }

    pub fn jogar(&mut self, posicao: usize, simbolo: Simbolo) -> bool {
        if posicao >= 9
            || self.resultado.is_some()
            || self.vez != simbolo
            || self.tabuleiro[posicao].is_some()
        {
            return false;
        }

        self.tabuleiro[posicao] = Some(simbolo);
        self.resultado = self.verificar_resultado();
        self.vez = simbolo.oposto();
        true
    }

    fn verificar_resultado(&self) -> Option<Resultado> {
        for [a, b, c] in LINHAS {
            if let Some(simbolo) = self.tabuleiro[a] {
                if self.tabuleiro[b] == Some(simbolo) && self.tabuleiro[c] == Some(simbolo) {
                    return Some(Resultado::Vitoria(simbolo));
                }
            }
        }

        if self.tabuleiro.iter().all(|casa| casa.is_some()) {
            Some(Resultado::Empate)
        } else {
            None
        }
    }
}