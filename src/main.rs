use ascn_rs::{outcome::Outcome, reader::Reader, writer::Writer};
use chess::{BitBoard, Board, BoardStatus, ChessMove, Color, Piece};
use clap::Parser;
use pgn_rs::{PGNReader, Visitor};
use std::{
    fs::{read_to_string, File},
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Parser, Debug)]
struct Arguments {
    input: PathBuf,
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
enum Format {
    ASCN,
    PGN,
}

impl Format {
    pub fn opposite(&self) -> Self {
        match self {
            Self::ASCN => Self::PGN,
            Self::PGN => Self::ASCN,
        }
    }

    pub fn get_extension(&self) -> &'static str {
        match self {
            Self::ASCN => "ascn",
            Self::PGN => "pgn",
        }
    }
}

struct PGNVisitor {
    board: Board,
    writer: Writer,
    outcome: Option<Outcome>,
}

impl PGNVisitor {
    fn new() -> Self {
        Self {
            board: Board::default(),
            writer: Writer::new(),
            outcome: None,
        }
    }
}

impl<'a> Visitor<'a> for PGNVisitor {
    fn start_game(&mut self) {
        self.board = Board::default();
        self.writer = Writer::new();
    }

    fn end_game(&mut self, outcome: &str) {
        self.outcome = Some(Outcome::from_string(outcome))
    }

    fn header(&mut self, _header: pgn_rs::Header) {}

    fn san(&mut self, san: pgn_rs::san::SAN) {
        let chess_move = san.to_move(&self.board);
        self.writer.add_move(&chess_move, &self.board);
        self.board = self.board.make_move_new(chess_move);
    }
}

fn main() {
    let args = Arguments::parse();

    let input_format = match args
        .input
        .extension()
        .expect("Could not determine input file type")
        .to_str()
        .expect("Could not parse extension")
    {
        "ascn" => Format::ASCN,
        "pgn" => Format::PGN,
        _ => panic!("Unknown input file type"),
    };

    let mut output_file = File::create(
        args.output.unwrap_or(
            args.input
                .with_extension(input_format.opposite().get_extension()),
        ),
    )
    .expect("Could not create output file");

    if input_format == Format::PGN {
        let data = read_to_string(args.input).expect("Could not find/read input file");

        let mut visitor = PGNVisitor::new();
        let reader = PGNReader::new(&data);
        reader.read(&mut visitor);

        output_file
            .write(&visitor.writer.get_data(visitor.outcome))
            .expect("Could not write to the output file");
        output_file
            .flush()
            .expect("Could not write to the output file");
    } else {
        let mut movetext = String::new();

        let mut input_file = File::open(args.input).expect("Could not find input file");
        let mut input_buf = Vec::new();
        input_file
            .read_to_end(&mut input_buf)
            .expect("Could not read input file");

        let mut reader = Reader::new(&input_buf);

        let mut current_move = 1u32;
        let mut board = Board::default();

        while let Some((chess_move, new_board)) = reader.next() {
            // dbg!(chess_move.get_dest().to_string());
            let san = get_san(chess_move, &board);

            if board.side_to_move() == Color::White {
                movetext += &format!("{}. {} ", current_move, san);
                current_move += 1;
            } else {
                movetext += &format!("{} ", san)
            }

            board = new_board;
        }

        let outcome = reader
            .get_outcome()
            .as_ref()
            .unwrap_or(&Outcome::Unknown)
            .to_string();

        write!(
            output_file,
            "[Result \"{}\"]\n\n{}{}",
            outcome, movetext, outcome
        )
        .expect("Could not write to the output file");
        output_file
            .flush()
            .expect("Could not write to the output file");
    }
}

fn get_piece(piece: Piece) -> &'static str {
    match piece {
        Piece::Pawn => "P",
        Piece::Knight => "N",
        Piece::Bishop => "B",
        Piece::Rook => "R",
        Piece::Queen => "Q",
        Piece::King => "K",
    }
}

fn get_san(chess_move: ChessMove, board: &Board) -> String {
    let new_board = board.make_move_new(chess_move);

    let mut result = get_piece(board.piece_on(chess_move.get_source()).unwrap()).to_string()
        + &chess_move.get_source().to_string();

    if board.piece_on(chess_move.get_dest()).is_some() {
        result += "x"
    }

    result += &chess_move.get_dest().to_string();

    if let Some(promotion) = chess_move.get_promotion() {
        result += "=";
        result += get_piece(promotion);
    }

    if new_board.status() == BoardStatus::Checkmate {
        result += "#"
    } else if new_board.checkers() != &BitBoard(0) {
        result += "+"
    }

    result
}
