use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{Color, Print, SetBackgroundColor, SetForegroundColor, Stylize},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand, QueueableCommand,
};
use rand::prelude::*;
use std::{
    fs,
    io::{self, stdout, Write},
    sync::{Arc, Mutex},
    thread::{self, sleep, JoinHandle},
    time, vec,
};

fn main() -> io::Result<()> {
    let mut stdout = stdout();

    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?.execute(Hide)?;

    let mut game = Game::new();
    game.run()?;

    stdout.execute(LeaveAlternateScreen)?.execute(Show)?;
    disable_raw_mode()?;
    Ok(())
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum State {
    Main,
    Playing,
    Paused,
    GameOver,
    Quit,
}
impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Playing => write!(f, "{}", "Playing"),
            State::GameOver => write!(f, "{}", "GameOver"),
            State::Main => write!(f, "{}", "Main"),
            State::Paused => write!(f, "{}", "Paused"),
            State::Quit => write!(f, "{}", "Quit"),
        }
    }
}

struct Player {
    x: u16,
    y: u16,
}
impl Player {
    fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

struct Missile {
    x: u16,
    y: Arc<Mutex<u16>>,
}
impl Missile {
    fn new(x: u16, y: u16) -> Self {
        Self {
            x,
            y: Arc::new(Mutex::new(y)),
        }
    }

    fn fire(&mut self) {
        let y_clone = self.y.clone();
        thread::spawn(move || loop {
            sleep(time::Duration::from_millis(20));
            let mut y = y_clone.lock().unwrap();
            if *y <= 0 {
                break;
            }
            *y -= 1;
        });
    }
}

struct Game {
    scene: Scene,
    state: Arc<Mutex<State>>,
    frame: Arc<Mutex<usize>>,
    player: Arc<Mutex<Player>>,
    missiles: Arc<Mutex<Vec<Missile>>>,
}
impl Game {
    fn new() -> Self {
        Self {
            scene: Scene::make(),
            state: Arc::new(Mutex::new(State::Main)),
            frame: Arc::new(Mutex::new(0)),
            player: Arc::new(Mutex::new(Player::new(0, 0))),
            missiles: Arc::new(Mutex::new(vec![])),
        }
    }

    fn run(&mut self) -> io::Result<()> {
        let frame_couter_join_handle = self.start_frame_counter();
        let listen_events_join_handle = self.listen_events();
        loop {
            let state = *self.state.lock().unwrap();
            match state {
                State::Main => {
                    sleep(time::Duration::from_millis(100));
                    self.render_main()?;
                }
                State::Paused => {
                    sleep(time::Duration::from_millis(100));
                    self.render_paused()?;
                }
                State::Playing => {
                    // sleep(time::Duration::from_millis(u64::from(BASE_SPEED_DELAY_IN_MILLIS / u64::from(*self.speed_rate.lock().unwrap()))));
                    sleep(time::Duration::from_millis(100));
                    self.render_playing()?;
                }
                State::GameOver => {
                    sleep(time::Duration::from_millis(100));
                    self.render_gameover()?;
                }
                State::Quit => {
                    break;
                }
                _ => {}
            }
        }

        let _ = frame_couter_join_handle.join();
        let _ = listen_events_join_handle.join();
        Ok(())
    }

    fn start_frame_counter(&mut self) -> JoinHandle<()> {
        let frame_clone = self.frame.clone();
        let state_clone = self.state.clone();
        let join_handle = thread::spawn(move || loop {
            // sleep(time::Duration::from_millis(u64::from(BASE_SPEED_DELAY_IN_MILLIS / u64::from(*speed_rate.lock().unwrap()))));
            sleep(time::Duration::from_millis(100));
            let state = state_clone.lock().unwrap();
            if *state == State::Quit {
                break;
            }
            if *state == State::Playing {
                let mut frame = frame_clone.lock().unwrap();
                *frame += 1;
            }
        });
        join_handle
    }

    fn listen_events(&self) -> JoinHandle<()> {
        let duration = time::Duration::from_millis(250);
        let state = self.state.clone();
        let player = self.player.clone();
        let missiles_clone = self.missiles.clone();
        let join_handle = thread::spawn(move || loop {
            if poll(duration).expect("Failed to poll event.") {
                let event = read().expect("Failed to read event.");
                let mut state = state.lock().unwrap();
                match event {
                    Event::Key(KeyEvent {
                        modifiers: KeyModifiers::CONTROL,
                        code: KeyCode::Char('c'),
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        *state = State::Quit;
                        break;
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('p'),
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        if *state == State::Playing {
                          *state = State::Paused;
                        } else if (*state == State::Paused) {
                            *state = State::Playing;
                        }
                    }
                    Event::Key(KeyEvent {
                        kind: KeyEventKind::Press,
                        ..
                    }) if *state == State::Main => {
                        let (cols, rows) = size().expect("Failed to get terminal size.");
                        let middle = cols / 2;
                        let mut player = player.lock().unwrap();
                        player.x = middle;
                        player.y = rows - 1;
                        *state = State::Playing;
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Left,
                        kind: KeyEventKind::Press,
                        ..
                    }) if *state == State::Playing => {
                        let mut player = player.lock().unwrap();
                        player.x -= 1;
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Right,
                        kind: KeyEventKind::Press,
                        ..
                    }) if *state == State::Playing => {
                        let mut player = player.lock().unwrap();
                        player.x += 1;
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Char(' '),
                        kind: KeyEventKind::Press,
                        ..
                    }) if *state == State::Playing => {
                        let player = player.lock().unwrap();
                        let x = player.x;
                        let y = player.y - 1;
                        let mut missile = Missile::new(x, y);
                        missile.fire();
                        let missiles = &mut missiles_clone.lock().unwrap();
                        missiles.push(missile);
                    }
                    _ => {}
                }
            }
        });
        join_handle
    }

    fn render_main(&self) -> io::Result<()> {
        let mut stdout = stdout();

        let s = format!(
            "{}\r\n\r\n{}\r\n\r\n\r\n\r\n{}",
            "River Raid Game".yellow(),
            "Help: (ctrl+c) Exit   (p) Pause",
            "Press any key to start..."
        );

        stdout
            .queue(Clear(ClearType::All))?
            .queue(MoveTo(0, 0))?
            .queue(Print(s))?;

        stdout.flush()?;

        Ok(())
    }

    fn render_paused(&self) -> io::Result<()> {
        let mut stdout = stdout();
        let (width, heigth) = size()?;

        let s = "Game Paused".on_white().black().bold();

        let x = width / 2 - 11 / 2;
        let y = heigth / 2;

        stdout.queue(MoveTo(x, y))?.queue(Print(s))?;
        stdout.flush()?;

        Ok(())
    }

    fn render_gameover(&self) -> io::Result<()> {
        let mut stdout = stdout();

        let s = format!(
            "{}\r\n\r\n{}",
            "Game Over!".red(),
            "Press ctrl+c to exit.."
        );

        stdout
            .queue(Clear(ClearType::All))?
            .queue(SetBackgroundColor(Color::Reset))?
            .queue(MoveTo(0, 0))?
            .queue(Print(s))?;

        stdout.flush()?;

        Ok(())
    }


    fn render_playing(&mut self) -> io::Result<()> {
        let mut stdout = stdout();
        let (_, rows) = size()?;

        let frame_index = self.frame.lock().unwrap();

        let mut scene = self.scene.get_current_scene(*frame_index, rows);
        scene.reverse();

        for (row_index, line) in scene.iter_mut().enumerate() {
            let y = row_index
                .try_into()
                .expect("Failed to conver usize to u16.");
            for (col_index, cell) in line.iter_mut().enumerate() {
                let x = col_index
                    .try_into()
                    .expect("Failed to convert usize to u16.");
                stdout.queue(MoveTo(x, y))?;
                match cell.kind {
                    Kind::LAND => {
                        stdout
                            .queue(SetForegroundColor(Color::Green))?
                            .queue(Print("█"))?;
                    }
                    Kind::RIVER => {
                        stdout
                            .queue(SetForegroundColor(Color::Blue))?
                            .queue(Print("█"))?;
                    }
                    Kind::ENEMY => {
                        stdout
                            .queue(SetForegroundColor(Color::White))?
                            .queue(SetBackgroundColor(Color::Blue))?
                            .queue(Print("✈"))?;
                    }
                    _ => {}
                }

                // render missiles and check collision with emenies
                let missiles = &mut self.missiles.lock().unwrap();
                let mut missile_indexes_to_remove = vec![];
                for (missile_index, missile) in missiles.iter_mut().enumerate() {
                    let missile_x = missile.x;
                    let missile_y = missile.y.lock().unwrap();

                    stdout
                        .queue(MoveTo(missile_x, *missile_y))?
                        .queue(SetForegroundColor(Color::Red))?
                        .queue(SetBackgroundColor(Color::Blue))?
                        .queue(Print("🭯"))?;
                    stdout.flush()?;

                    if cell.kind == Kind::ENEMY && missile_x == x && *missile_y <= y {
                        missile_indexes_to_remove.push(missile_index);
                        cell.kind = Kind::RIVER;
                    }
                    if *missile_y == 0 {
                        missile_indexes_to_remove.push(missile_index);
                    }
                }
                for index in missile_indexes_to_remove.iter() {
                    missiles.remove(*index);
                }
            }
        }

        // render player
        let player = &self.player.lock().unwrap();
        stdout
            .queue(MoveTo(player.x, player.y))?
            .queue(SetBackgroundColor(Color::Blue))?
            .queue(SetForegroundColor(Color::Black))?
            .queue(Print("🛦"))?;

        stdout.flush()?;

        // checking player collision with enemy or land
        let scene_player_match_cell = {
            let row = scene.get(usize::from(player.y)).unwrap();
            row.get(usize::from(player.x)).unwrap()
        };
        if scene_player_match_cell.kind == Kind::LAND || scene_player_match_cell.kind == Kind::ENEMY
        {
            let state_cloned = self.state.clone();
            let mut state = state_cloned.lock().unwrap();
            *state = State::GameOver;
        }

        Ok(())
    }
}

struct Scene {
    cells: Vec<Vec<Cell>>,
}
impl Scene {
    fn make() -> Self {
        let mut result: Vec<Vec<Cell>> = vec![];
        let (terminal_width, _) = size().expect("Failed to get terminal size.");
        let designs: Vec<Vec<f64>> = fs::read_to_string("scene.design")
            .expect("Failed to read design file.")
            .lines()
            .into_iter()
            .map(|line| {
                return line
                    .split(' ')
                    .into_iter()
                    .map(|item| item.parse::<f64>().unwrap())
                    .collect();
            })
            .collect();

        for design_index in 0..designs.len() {
            let design = designs.get(design_index).unwrap();
            let (mut line, part_height) = Self::generate_line(design, terminal_width);

            // generate first scene of the game without enemies
            let mut has_enemy = true;
            if design_index == 0 {
                has_enemy = false;
            }
            let mut part = Self::generate_with_height(&mut line, part_height, has_enemy);
            result.append(&mut part);
        }

        Self { cells: result }
    }

    fn percent_to_terminal_size(percent: &f64, terminal_width: u16) -> usize {
        f64::floor(percent * f64::from(terminal_width) / 100.0) as usize
    }

    fn generate_line(design: &Vec<f64>, terminal_width: u16) -> (Vec<Cell>, usize) {
        let land_part_one_size =
            Self::percent_to_terminal_size(design.get(0).unwrap(), terminal_width);
        let river_part_one_size =
            Self::percent_to_terminal_size(design.get(1).unwrap(), terminal_width);
        let land_part_two_size =
            Self::percent_to_terminal_size(design.get(2).unwrap(), terminal_width);
        let river_part_two_size =
            Self::percent_to_terminal_size(design.get(3).unwrap(), terminal_width);
        let mut land_part_three_size =
            Self::percent_to_terminal_size(design.get(4).unwrap(), terminal_width);
        let total_size = land_part_one_size
            + river_part_one_size
            + land_part_two_size
            + river_part_two_size
            + land_part_three_size;
        if total_size < usize::from(terminal_width) {
            land_part_three_size = land_part_three_size + usize::from(terminal_width) - total_size;
        }
        let part_height = *design.get(5).unwrap() as usize;

        let mut v = vec![];
        let mut land_part_one = Cell::create_cells_vec(land_part_one_size, Kind::LAND);
        let mut river_part_one = Cell::create_cells_vec(river_part_one_size, Kind::RIVER);
        let mut land_part_two = Cell::create_cells_vec(land_part_two_size, Kind::LAND);
        let mut river_part_two = Cell::create_cells_vec(river_part_two_size, Kind::RIVER);
        let mut land_part_three = Cell::create_cells_vec(land_part_three_size, Kind::LAND);

        v.append(&mut land_part_one);
        v.append(&mut river_part_one);
        v.append(&mut land_part_two);
        v.append(&mut river_part_two);
        v.append(&mut land_part_three);

        (v, part_height)
    }

    fn generate_with_height(
        line: &mut Vec<Cell>,
        height: usize,
        with_enemy: bool,
    ) -> Vec<Vec<Cell>> {
        let mut result: Vec<Vec<Cell>> = vec![];
        if with_enemy {
            let river_indexes: Vec<usize> = line
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| {
                    if cell.kind == Kind::RIVER {
                        return Some(index);
                    } else {
                        return None;
                    }
                })
                .collect();
            for _ in 0..height {
                let mut line = line.clone();
                let mut rng = thread_rng();
                let enemy_posibility = rng.gen_bool(1.0 / 2.0);
                if enemy_posibility {
                    let enemy_index = river_indexes
                        .get(rng.gen_range(0..river_indexes.len()))
                        .unwrap()
                        .clone();
                    let cell = line.get_mut(enemy_index).unwrap();
                    *cell = Cell { kind: Kind::ENEMY };
                }
                result.push(line.clone());
            }
        } else {
            for _ in 0..height {
                result.push(line.clone());
            }
        }
        result
    }

    fn get_current_scene(&mut self, index: usize, height_size: u16) -> Vec<&mut Vec<Cell>> {
        let len = self.cells.len();

        let start = index % len;
        let end = (index + usize::from(height_size)) % len;

        let cells_iter_mut = self.cells.iter_mut().enumerate();
        let mut chunk: Vec<&mut Vec<Cell>> = vec![];
        if end > start {
            for (index, vec) in cells_iter_mut {
                if index >= start && index <= end {
                    chunk.push(vec);
                }
            }
        } else {
            let mut first_chunk: Vec<&mut Vec<Cell>> = vec![];
            let mut second_chunk: Vec<&mut Vec<Cell>> = vec![];
            for (index, vec) in cells_iter_mut {
                if index <= end {
                    second_chunk.push(vec);
                } else if index >= start && index <= len {
                    first_chunk.push(vec);
                }
            }
            chunk.append(&mut first_chunk);
            chunk.append(&mut second_chunk);
        };
        chunk
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    LAND,
    RIVER,
    ENEMY,
}
#[derive(Clone, Copy)]
struct Cell {
    kind: Kind,
}
impl Cell {
    fn create_cells_vec(size: usize, kind: Kind) -> Vec<Cell> {
        let cell = Cell { kind };
        vec![cell; size as usize]
    }
}
