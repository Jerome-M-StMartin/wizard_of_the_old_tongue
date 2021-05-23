//Jerome M. St.Martin
//05/19/2021

use std::rc::Rc;
use std::any::Any;
use std::cell::Cell;

use bracket_terminal::prelude::{BTerm, Point};

use super::look_n_feel::{ColorOption, Dir};
use super::drawable::Drawable;
use super::observer::{Observer, Observable};
use super::command::{Command, Commandable};
use super::user_input::{InputEvent, UserInput};

#[derive(PartialEq, Copy, Clone)]
pub enum Selection { NewGame, LoadGame, Quit }

pub struct MainMenu {
    pos: Point,
    selection: Cell<Selection>,

    observer_id: usize,
    user_input: Rc<dyn Observable>,
}

impl MainMenu {
    pub fn new(user_input: Rc<UserInput>) -> Self {
        MainMenu {
            pos: Point {x:0,y:0},
            selection: Cell::new(Selection::NewGame),
            observer_id: user_input.id_gen.generate_observer_id(),
            user_input,
        }
    }
    fn change_selection(&self, direction: Dir) {
        match (direction, self.selection.get()) {
            (Dir::UP, Selection::NewGame) => self.selection.set(Selection::Quit),
            (Dir::RIGHT, Selection::NewGame) => self.selection.set(Selection::Quit),

            (Dir::UP, Selection::LoadGame) => self.selection.set(Selection::NewGame),
            (Dir::RIGHT, Selection::LoadGame) => self.selection.set(Selection::NewGame),

            (Dir::UP, Selection::Quit) => self.selection.set(Selection::LoadGame),
            (Dir::RIGHT, Selection::Quit) => self.selection.set(Selection::LoadGame),

            (Dir::DOWN, Selection::NewGame) => self.selection.set(Selection::LoadGame),
            (Dir::LEFT, Selection::NewGame) => self.selection.set(Selection::LoadGame),

            (Dir::DOWN, Selection::LoadGame) => self.selection.set(Selection::Quit),
            (Dir::LEFT, Selection::LoadGame) => self.selection.set(Selection::Quit),

            (Dir::DOWN, Selection::Quit) => self.selection.set(Selection::NewGame),
            (Dir::LEFT, Selection::Quit) => self.selection.set(Selection::NewGame),
        };
    }
    fn select(&self) -> Selection {
        self.selection.get()
    }
}

impl Drawable for MainMenu {
    fn draw(&self, ctx: &mut BTerm) {
        //let save_exists = saveload_system::does_save_exist();
        ctx.print_color_centered(15, ColorOption::DEFAULT.value(), ColorOption::NONE.value(), "GoblinRL");
        ctx.print_color_centered(24, ColorOption::DEFAULT.value(), ColorOption::NONE.value(), "New Game");
        ctx.print_color_centered(25, ColorOption::DEFAULT.value(), ColorOption::NONE.value(), "Load Game");
        ctx.print_color_centered(26, ColorOption::DEFAULT.value(), ColorOption::NONE.value(), "Quit Game");
        match self.selection.get() {
            Selection::NewGame => {
                ctx.print_color_centered(24, ColorOption::FOCUS.value(), ColorOption::NONE.value(), "New Game");
            },
            Selection::LoadGame => {
                ctx.print_color_centered(25, ColorOption::FOCUS.value(), ColorOption::NONE.value(), "Load Game");
            },
            Selection::Quit => {
                ctx.print_color_centered(26, ColorOption::FOCUS.value(), ColorOption::NONE.value(), "Quit Game");
            }
        }
    }
    fn move_to(&self, pos: Point) {
        //self.pos = pos;
    }
    fn orth_move(&self, direction: Dir) {
        /*match direction {
            Dir::UP => self.pos.y -= 1,
            Dir::DOWN => self.pos.y += 1,
            Dir::LEFT => self.pos.x -= 1,
            Dir::RIGHT => self.pos.x += 1,
            _ => {},
        }*/
    }
}



//===================================
//======== Observer Pattern =========
//===================================
impl Observer for MainMenu {
    fn id(&self) -> usize { self.observer_id }
    fn update(&self) {
        let observable = self.user_input.as_any().downcast_ref::<UserInput>();
        if let Some(user_input) = observable {
            if let Some(input_event) = user_input.input.get() {
                match input_event {
                    InputEvent::HJKL(dir) | InputEvent::WASD(dir) => {
                        self.send(Box::new(ChangeSelectionCommand::new(dir)));
                    },
                    InputEvent::ENTER => {
                        self.send(Box::new(SelectCommand::new()));
                    }
                    _ => {},
                }
            }
        }
    }
    fn setup_cursor(&self) {} //this needs to leave this trait
}


//===================================
//======== Command Pattern ==========
//===================================
impl Commandable<MainMenu> for MainMenu {
    fn send(&self, cmd: Box<dyn Command<MainMenu>>) {
        cmd.execute(self);
    }
}

//======= Commands =======
struct ChangeSelectionCommand {
    direction: Dir,
}
impl ChangeSelectionCommand {
    pub fn new(direction: Dir) -> Self {
        ChangeSelectionCommand { direction }
    }
}
impl Command<MainMenu> for ChangeSelectionCommand {
    fn execute(&self, main_menu: &MainMenu) {
        main_menu.change_selection(self.direction);
    }
    fn as_any(&self) -> &dyn Any { self }
}

struct SelectCommand {}
impl SelectCommand {
    pub fn new() -> Self {
        SelectCommand {}
    }
}
impl Command<MainMenu> for SelectCommand {
    fn execute(&self, main_menu: &MainMenu) {
        main_menu.select();
    }
    fn as_any(&self) -> &dyn Any { self }
}
