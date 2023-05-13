use core::fmt;
use std::io::stdout;
use std::time::{Duration, Instant};

use crossterm::event::KeyModifiers;
use crossterm::style::Color;
use crossterm::{cursor, QueueableCommand};

use rand::seq::SliceRandom;

use rand::{
    distributions::{Distribution, Standard},
    rngs::ThreadRng,
    Rng,
};

const STACK_NUM_COLS: usize = 10;
const STACK_NUM_ROWS: usize = 20;

// One entry per tetromino, discribing each 4 rotation states by the relative position of the minos
// The first state is the spawning state, and thes tates are listed clock-wise.
const TETROMINO_DATA: [[[(i8, i8); 4]; 4]; 7] = [
    // I
    [
        [(0, -1), (1, -1), (2, -1), (3, -1)],
        [(2, 0), (2, -1), (2, -2), (2, -3)],
        [(0, -2), (1, -2), (2, -2), (3, -2)],
        [(1, 0), (1, -1), (1, -2), (1, -3)],
    ],
    // J
    [
        [(0, 0), (0, -1), (1, -1), (2, -1)],
        [(1, 0), (2, 0), (1, -1), (1, -2)],
        [(0, -1), (1, -1), (2, -1), (2, -2)],
        [(1, 0), (1, -1), (0, -2), (1, -2)],
    ],
    // L
    [
        [(2, 0), (0, -1), (1, -1), (2, -1)],
        [(1, 0), (1, -1), (1, -2), (2, -2)],
        [(0, -1), (1, -1), (2, -1), (0, -2)],
        [(0, 0), (1, 0), (1, -1), (1, -2)],
    ],
    // O
    [[(1, 0), (2, 0), (1, -1), (2, -1)]; 4],
    // S
    [
        [(1, 0), (2, 0), (0, -1), (1, -1)],
        [(1, 0), (1, -1), (2, -1), (2, -2)],
        [(1, -1), (2, -1), (0, -2), (1, -2)],
        [(0, 0), (0, -1), (1, -1), (1, -2)],
    ],
    // T
    [
        [(1, 0), (0, -1), (1, -1), (2, -1)],
        [(1, 0), (1, -1), (2, -1), (1, -2)],
        [(0, -1), (1, -1), (2, -1), (1, -2)],
        [(1, 0), (0, -1), (1, -1), (1, -2)],
    ],
    // Z
    [
        [(0, 0), (1, 0), (1, -1), (2, -1)],
        [(2, 0), (1, -1), (2, -1), (1, -2)],
        [(0, -1), (1, -1), (1, -2), (2, -2)],
        [(1, 0), (0, -1), (1, -1), (0, -2)],
    ],
];

//#[derive(Clone, Copy, PartialEq)]
//struct Color(u8);

#[derive(Clone, Copy, PartialEq)]
enum Mino {
    Free,
    Occupied(Color),
    PendingClear,
}

#[derive(Copy, Clone, Debug)]
enum Tetromino {
    I,
    J,
    L,
    O,
    S,
    T,
    Z,
}

#[derive(Copy, Clone)]
struct RotationState(u8);

impl RotationState {
    fn cw(self) -> Self {
        RotationState((self.0 + 1) % 4)
    }

    fn ccw(self) -> Self {
        RotationState((self.0 + 3) % 4)
    }
}

impl Default for RotationState {
    fn default() -> Self {
        Self(0)
    }
}

impl From<RotationState> for usize {
    fn from(value: RotationState) -> Self {
        value.0 as usize
    }
}

impl<T> From<T> for Tetromino
where
    T: std::borrow::Borrow<usize>,
{
    fn from(value: T) -> Self {
        match *value.borrow() % 7 {
            0 => Tetromino::I,
            1 => Tetromino::J,
            2 => Tetromino::L,
            3 => Tetromino::O,
            4 => Tetromino::S,
            5 => Tetromino::T,
            6 => Tetromino::Z,
            _ => panic!("bad value"),
        }
    }
}

impl fmt::Display for Tetromino {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let letters = "IJLOSTZ";
        write!(f, "{}", letters.chars().nth(*self as usize).unwrap())
    }
}

impl Distribution<Tetromino> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Tetromino {
        rng.gen::<usize>().into()
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum State {
    Spawn,
    Fall,
    Lock,
    ClearRows,
    Paused,
    End,
}

struct Ttrys {
    cur_tetro: Option<Tetromino>,
    cur_position: (i8, i8),
    cur_state: RotationState,
    hard_drop: bool,
    clear_rows: Vec<i8>,
    score: u32,
    level: u32,
    state: State,
    saved_state: State,
    stack: [Mino; STACK_NUM_COLS * STACK_NUM_ROWS],
    stack_height: i8,
    sequence: TetrominoSequence,
}

impl Ttrys {
    fn new() -> Self {
        Ttrys {
            cur_tetro: None,
            cur_position: (0, 0),
            cur_state: RotationState::default(),
            hard_drop: false,
            clear_rows: Vec::new(),
            score: 0,
            level: 0,
            state: State::Spawn,
            saved_state: State::End,
            stack: [Mino::Free; STACK_NUM_COLS * STACK_NUM_ROWS],
            stack_height: 0,
            sequence: TetrominoSequence::new(5),
        }
    }

    #[allow(dead_code)]
    fn random_fill(&mut self) {
        for row in 0..STACK_NUM_ROWS {
            let mut rng = ThreadRng::default();
            for col in 0..STACK_NUM_COLS {
                let brick = if rng.gen_bool(0.3) {
                    let color = rng.gen();
                    Mino::Occupied(Color::AnsiValue(color))
                } else {
                    Mino::Free
                };
                self.stack[row * STACK_NUM_COLS + col] = brick;
            }
        }
    }

    fn clear_stack(&mut self) {
        self.stack.fill(Mino::Free);
        self.stack_height = 0;
    }

    // return whether to continue
    fn step(&mut self) -> bool {
        match self.state {
            State::Spawn => {
                self.cur_tetro = Some(self.sequence.pop());
                self.cur_position = (STACK_NUM_COLS as i8 / 2, (STACK_NUM_ROWS - 1) as i8);
                if let Some(Tetromino::I) = self.cur_tetro {
                    self.cur_position.1 += 1;
                }
                self.cur_state = RotationState::default();
                self.state = if self.collide(self.cur_state, (0, 0)) {
                    State::End
                } else {
                    State::Fall
                };
            }
            State::Fall => {
                if self.hard_drop {
                    let mut offset = -1;
                    while !self.collide(self.cur_state, (0, offset)) {
                        offset -= 1;
                    }
                    self.cur_position.1 += offset + 1;
                    self.state = State::Lock;
                } else {
                    if self.collide(self.cur_state, (0, -1)) {
                        self.state = State::Lock;
                    } else {
                        self.cur_position.1 -= 1;
                    }
                }
            }
            State::Lock => {
                let idx: usize = self.cur_tetro.unwrap() as usize;
                let state: usize = self.cur_state.into();

                // finalize the locked piece into the stack
                TETROMINO_DATA[idx][state]
                    .iter()
                    .map(|(x, y)| {
                        let h = self.cur_position.1 + y;
                        let idx = h as usize * STACK_NUM_COLS + (self.cur_position.0 + x) as usize;
                        self.stack_height = self.stack_height.max(h);
                        idx
                    })
                    .for_each(|idx| {
                        let block = &mut self.stack[idx];
                        *block = Mino::Occupied(tetro_color(self.cur_tetro.unwrap()));
                    });

                // list the full rows after locking the tetromino into the stack
                // for later removal
                let set: std::collections::HashSet<i8> = TETROMINO_DATA[idx][state]
                    .iter()
                    .map(|(_, offset)| self.cur_position.1 + offset)
                    .collect();
                self.clear_rows = set
                    .into_iter()
                    .filter(|&row| {
                        let start = (row as usize) * STACK_NUM_COLS;
                        let end = start + STACK_NUM_COLS;
                        self.stack[start..end]
                            .iter()
                            .fold(true, |full, block| full && *block != Mino::Free)
                    })
                    .collect();
                self.clear_rows.sort();
                if !self.clear_rows.is_empty() {
                    // Color full rows in a special way
                    self.clear_rows.iter().for_each(|&row| {
                        let start = (row as usize) * STACK_NUM_COLS;
                        let end = start + STACK_NUM_COLS;
                        self.stack[start..end]
                            .iter_mut()
                            .for_each(|block| *block = Mino::PendingClear)
                    });
                    self.state = State::ClearRows;
                } else {
                    self.state = State::Spawn;
                }

                self.hard_drop = false;
            }
            State::ClearRows => {
                // Drop rows down where cleared rows have left space.
                // gather the clear streaks (set of consecutives lines cleared) for later scoring
                let mut clear_streaks = Vec::new();
                // use the stack height as convenient sentinel
                self.clear_rows.push(self.stack_height + 1);

                let mut clear_it = self.clear_rows.iter();
                let mut clear_row = *clear_it.next().unwrap();
                let mut drop = 0;
                let mut streak = 0;
                for row in 0..=self.stack_height {
                    if row < clear_row {
                        if drop > 0 {
                            let src_blocks = (row as usize * STACK_NUM_COLS)
                                ..((row as usize + 1) * STACK_NUM_COLS);
                            let dst = (row - drop) as usize * STACK_NUM_COLS;
                            self.stack.copy_within(src_blocks, dst);
                            if streak > 0 {
                                clear_streaks.push(streak);
                                streak = 0;
                            }
                        }
                    } else {
                        clear_row = *clear_it.next().unwrap();
                        drop += 1;
                        streak += 1;
                    }
                }
                // the top rows now contains gabarge, clear them
                for r in 0..drop {
                    let h = self.stack_height;
                    let row_range = ((h - r) as usize * STACK_NUM_COLS)
                        ..((h - r + 1) as usize * STACK_NUM_COLS);
                    self.stack[row_range].fill(Mino::Free);
                }
                self.stack_height -= self.clear_rows.len() as i8 - 1;
                self.clear_rows.clear();
                self.state = State::Spawn;

                // update score
                for streak in clear_streaks {
                    self.score += self.clear_reward(streak);
                }
                self.level = self.score / 1000;
            }
            _ => (),
        }
        false
    }

    fn collide(&self, rotation: RotationState, offset: (i8, i8)) -> bool {
        if let Some(tetro) = self.cur_tetro {
            let idx = tetro as usize;
            let minos: [(i8, i8); 4] =
                TETROMINO_DATA[idx][<RotationState as Into<usize>>::into(rotation)];
            let x0 = self.cur_position.0 + offset.0;
            let y0 = self.cur_position.1 + offset.1;
            for mino in minos {
                let x = x0 + mino.0;
                let y = y0 + mino.1;
                if (0..STACK_NUM_COLS as i8).contains(&x) && (0..STACK_NUM_ROWS as i8).contains(&y)
                {
                    if self.stack[y as usize * STACK_NUM_COLS + x as usize] != Mino::Free {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }
        false
    }

    // Return potential wall kick offset
    fn test_rotation(&self, cw: bool) -> Option<(i8, i8)> {
        const WALL_KICK_OFFSETS: [[(i8, i8); 4]; 8] = [[(0, 0); 4]; 8];
        let (direction, next_state) = if cw {
            (0, self.cur_state.cw())
        } else {
            (1, self.cur_state.ccw())
        };
        let rotation_id: usize = self.cur_state.into();
        for offset in WALL_KICK_OFFSETS[2 * rotation_id + direction] {
            if !self.collide(next_state, offset) {
                return Some(offset);
            }
        }
        None
    }

    fn update(&mut self, action: UserAction) {
        match action {
            UserAction::MoveLeft => {
                if self.state != State::Fall {
                    return;
                }
                if !self.collide(self.cur_state, (-1, 0)) {
                    self.cur_position.0 = self.cur_position.0.saturating_sub(1);
                }
            }
            UserAction::MoveRight => {
                if self.state != State::Fall {
                    return;
                }
                if !self.collide(self.cur_state, (1, 0)) {
                    self.cur_position.0 += 1;
                }
            }
            UserAction::RotateCW | UserAction::RotateCCW => {
                if self.state != State::Fall {
                    return;
                }
                if let Some(offset) = self.test_rotation(action == UserAction::RotateCW) {
                    self.cur_state = self.cur_state.cw();
                    self.cur_position.0 += offset.0;
                    self.cur_position.1 += offset.1;
                }
            }
            UserAction::HardDrop => self.hard_drop = true,
            UserAction::Quit => {
                self.state = State::End;
            }
            UserAction::TogglePause => {
                if self.state == State::Paused {
                    self.state = self.saved_state;
                } else {
                    self.saved_state = self.state;
                    self.state = State::Paused;
                }
            }
            UserAction::ClearStack => self.clear_stack(),
            //_ => (),
        }
    }

    fn level(&self) -> u32 {
        self.level
    }

    fn score(&self) -> u32 {
        self.score
    }

    fn clear_reward(&self, combo_size: i8) -> u32 {
        let rewards = [100, 250, 500, 1000];
        rewards[(combo_size - 1).clamp(0, 3) as usize]
    }

    fn running(&self) -> bool {
        self.state != State::End
    }
}

struct TetrominoSequence {
    cur_tetro: Tetromino,
    bag: Vec<Tetromino>,
    bag_size: usize,
}

impl TetrominoSequence {
    fn new(bag_size: usize) -> Self {
        let bag_size = bag_size.max(1).min(7);
        let mut this = TetrominoSequence {
            cur_tetro: Tetromino::I,
            bag: Vec::with_capacity(bag_size),
            bag_size,
        };
        this.pop();
        this
    }

    fn peek(&self) -> Tetromino {
        self.cur_tetro
    }

    fn pop(&mut self) -> Tetromino {
        if self.bag.is_empty() {
            let mut ids: [usize; 7] = core::array::from_fn(|i| (i + 1) as usize);
            ids.partial_shuffle(&mut ThreadRng::default(), self.bag_size)
                .0
                .iter()
                .for_each(|idx| self.bag.push(idx.into()));
        }
        let ret = self.cur_tetro;
        self.cur_tetro = self.bag.pop().unwrap();
        ret
    }
}

#[derive(PartialEq, Debug)]
enum UserAction {
    MoveLeft,
    MoveRight,
    RotateCW,
    RotateCCW,
    HardDrop,
    //SoftDrop,
    TogglePause,
    ClearStack, // hack
    Quit,
}

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> RawModeGuard {
        use crossterm::terminal::enable_raw_mode;
        enable_raw_mode().ok();
        RawModeGuard
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        use crossterm::terminal::disable_raw_mode;
        disable_raw_mode().ok();
    }
}

fn get_user_action(timeout: &Timeout) -> Option<UserAction> {
    use crossterm::event::{poll, read, Event, KeyCode};

    let _raw_mode = RawModeGuard::new();

    poll(timeout.remaining()).map_or(None, |has_event| {
        if has_event {
            read().map_or(None, |event| match event {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Left => Some(UserAction::MoveLeft),
                    KeyCode::Right => Some(UserAction::MoveRight),
                    KeyCode::Up => Some(UserAction::RotateCW),
                    KeyCode::Down => Some(UserAction::RotateCCW),
                    KeyCode::Char(' ') => Some(UserAction::HardDrop),
                    KeyCode::Char('p') => Some(UserAction::TogglePause),
                    KeyCode::Char('x') => Some(UserAction::ClearStack),
                    KeyCode::Esc | KeyCode::Char('q') => Some(UserAction::Quit),
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        Some(UserAction::Quit)
                    }
                    _ => None,
                },
                _ => None,
            })
        } else {
            None
        }
    })
}

struct Timeout {
    start: Instant,
    duration: Duration,
}

impl Timeout {
    fn new(duration: Duration) -> Self {
        Timeout {
            start: Instant::now(),
            duration,
        }
    }

    fn remaining(&self) -> Duration {
        if self.expired() {
            Duration::default()
        } else {
            self.duration - self.start.elapsed()
        }
    }

    fn expired(&self) -> bool {
        self.start.elapsed() > self.duration
    }
}

fn tetro_color(tetro: Tetromino) -> Color {
    match tetro {
        Tetromino::I => Color::Cyan,
        Tetromino::J => Color::Blue,
        Tetromino::L => Color::AnsiValue(214),
        Tetromino::O => Color::Yellow,
        Tetromino::S => Color::Green,
        Tetromino::T => Color::Magenta,
        Tetromino::Z => Color::Red,
    }
}

struct GameScreen;

impl GameScreen {
    fn new() -> Self {
        let mut stdout = stdout();
        stdout.queue(cursor::Hide).ok();
        GameScreen
    }

    fn draw(&self, ttrys: &Ttrys) -> crossterm::Result<std::io::Stdout> {
        use crossterm::style;
        use std::io::Write;

        let padding_left = 5;

        let mut s = stdout();

        // stack top
        s.queue(cursor::MoveToColumn(padding_left))?;
        s.queue(style::Print("╔"))?;
        let horiz_border = "═".repeat(2);
        for _ in 0..STACK_NUM_COLS {
            s.queue(style::Print(&horiz_border))?;
        }
        s.queue(style::Print("╗\n"))?;

        // stack content
        for row in (0..STACK_NUM_ROWS).rev() {
            s.queue(cursor::MoveToColumn(padding_left))?;
            s.queue(style::Print("║"))?;
            for col in 0..STACK_NUM_COLS {
                let block = ttrys.stack[row * STACK_NUM_COLS + col];
                match block {
                    Mino::Occupied(color) => {
                        s.queue(style::SetBackgroundColor(color))?;
                        s.queue(style::Print("  "))?;
                        s.queue(style::ResetColor)?;
                    }
                    Mino::PendingClear => {
                        s.queue(style::SetBackgroundColor(Color::White))?;
                        s.queue(style::Print("<>"))?;
                        s.queue(style::ResetColor)?;
                    }
                    _ => {
                        s.queue(style::Print("  "))?;
                    }
                }
            }
            s.queue(style::Print("║\n"))?;
        }

        // stack bottom
        s.queue(cursor::MoveToColumn(padding_left))?;
        s.queue(style::Print("╚"))?;
        let horiz_border = "═".repeat(2);
        for _ in 0..STACK_NUM_COLS {
            s.queue(style::Print(&horiz_border))?;
        }
        s.queue(style::Print("╝"))?;

        // draw current tetromino
        s.queue(cursor::SavePosition)?;
        if let Some(tetro) = ttrys.cur_tetro {
            s.queue(cursor::MoveToPreviousLine(
                (ttrys.cur_position.1 + 1) as u16,
            ))?;
            s.queue(cursor::MoveToColumn(
                ((padding_left + 1) as i8 + (2 * ttrys.cur_position.0)) as u16,
            ))?;
            let position = cursor::position().unwrap();
            let minos: [(i8, i8); 4] = TETROMINO_DATA[tetro as usize]
                [<RotationState as Into<usize>>::into(ttrys.cur_state)];
            s.queue(style::SetBackgroundColor(tetro_color(tetro)))?;
            for mino in minos {
                if mino.0 > 0 {
                    s.queue(cursor::MoveRight(2 * mino.0 as u16))?;
                }
                if mino.1 < 0 {
                    s.queue(cursor::MoveDown(-(mino.1) as u16))?;
                }
                s.queue(style::Print("  "))?;
                s.queue(cursor::MoveTo(position.0, position.1))?;
            }
            s.queue(style::ResetColor)?;
        }
        s.queue(cursor::RestorePosition)?;

        // draw next tetromino
        s.queue(cursor::SavePosition)?;
        let tetro = ttrys.sequence.peek();
        s.queue(cursor::MoveToPreviousLine(STACK_NUM_ROWS as u16))?;
        s.queue(cursor::MoveToColumn(
            padding_left as u16 + 2 + 2 * STACK_NUM_COLS as u16 + 5,
        ))?;
        let position = cursor::position().unwrap();
        s.queue(style::ResetColor)?;
        for _ in 0..4 {
            s.queue(style::Print("        "))?;
            s.queue(cursor::MoveLeft(8))?;
            s.queue(cursor::MoveDown(1))?;
        }
        s.queue(cursor::MoveTo(position.0, position.1))?;

        let minos: [(i8, i8); 4] = TETROMINO_DATA[tetro as usize][0];
        s.queue(style::SetBackgroundColor(tetro_color(tetro)))?;
        for mino in minos {
            if mino.0 > 0 {
                s.queue(cursor::MoveRight(2 * mino.0 as u16))?;
            }
            if mino.1 < 0 {
                s.queue(cursor::MoveDown(-(mino.1) as u16))?;
            }
            s.queue(style::Print("  "))?;
            s.queue(cursor::MoveTo(position.0, position.1))?;
        }
        s.queue(style::ResetColor)?;
        s.queue(cursor::RestorePosition)?;

        // show score / level
        s.queue(cursor::SavePosition)?;
        s.queue(cursor::MoveToPreviousLine(3))?;
        s.queue(cursor::MoveToColumn(
            padding_left as u16 + 2 + 2 * STACK_NUM_COLS as u16 + 5,
        ))?;
        s.queue(style::Print(format!("Level: {:}", ttrys.level)))?;
        s.queue(cursor::MoveToColumn(
            padding_left as u16 + 2 + 2 * STACK_NUM_COLS as u16 + 5,
        ))?;
        s.queue(cursor::MoveDown(1))?;
        s.queue(style::Print(format!("Score: {:}", ttrys.score)))?;
        s.queue(cursor::RestorePosition)?;

        s.queue(cursor::MoveToPreviousLine((STACK_NUM_ROWS + 1) as u16))?;

        s.flush().ok();

        Ok(s)
    }
}

impl Drop for GameScreen {
    fn drop(&mut self) {
        let mut stdout = stdout();
        stdout.queue(cursor::Show).ok(); // TODO: panic in drop ?
    }
}

fn duration_from_level(level: u32) -> Duration {
    // the model is:
    //    * level base_level..=top_level: a power function with fixed power b
    //    * level < base_level or level > topLevel: constant function
    const BASE_LEVEL: f32 = 0.0;
    const TOP_LEVEL: f32 = 10.0;
    const B: f32 = 0.7;
    const MIN_DURATION: f32 = 150.0;
    const MAX_DURATION: f32 = 600.0;

    let level = level as f32;
    if level < BASE_LEVEL {
        Duration::from_millis(MAX_DURATION as _)
    } else if level > TOP_LEVEL {
        Duration::from_millis(MIN_DURATION as _)
    } else {
        let a = (MAX_DURATION - MIN_DURATION) / (BASE_LEVEL.powf(B) - TOP_LEVEL.powf(B));
        let c = ((MAX_DURATION * TOP_LEVEL.powf(B)) - (MIN_DURATION * BASE_LEVEL.powf(B)))
            / (TOP_LEVEL.powf(B) - BASE_LEVEL.powf(B));
        let millis = a * level.powf(B) + c;
        Duration::from_millis(millis as _)
    }
}

fn main() {
    let mut ttrys = Ttrys::new();
    let display = GameScreen::new();

    let mut timeout = Timeout::new(Duration::default());
    while ttrys.running() {
        display.draw(&ttrys).ok();
        while !timeout.expired() {
            if let Some(action) = get_user_action(&timeout) {
                ttrys.update(action);
                break;
            }
        }
        if timeout.expired() {
            let step_duration = duration_from_level(ttrys.level());
            timeout = Timeout::new(step_duration);
            ttrys.step();
        }
    }
    //display.clean_up();
    println!("Game over ! {} pts\x1b[0K", ttrys.score());
}
