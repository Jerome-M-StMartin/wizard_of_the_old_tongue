extern crate serde;

use rltk::{GameState, Rltk, Point, VirtualKeyCode};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};
mod components;
pub use components::*;
mod map;
pub use map::*;
mod player;
use player::*;
mod rect;
pub use rect::Rect;
mod visibility_system;
use visibility_system::VisibilitySystem;
mod hostile_ai_system;
use hostile_ai_system::HostileAI;
mod map_indexing_system;
use map_indexing_system::MapIndexingSystem;
mod melee_combat_system;
use melee_combat_system::MeleeCombatSystem;
mod damage_system;
use damage_system::DamageSystem;
mod gui;
mod gamelog;
mod spawner;
mod inventory_system;
use inventory_system::ItemCollectionSystem;
use inventory_system::ItemUseSystem;
use inventory_system::ItemDropSystem;
mod equip_system;
use equip_system::EquipSystem;
pub mod saveload_system;
mod random_table;
mod c_menu_system;
use c_menu_system::ContextMenuSystem;
mod healing_system;
use healing_system::HealingSystem;
mod bleed_system;
use bleed_system::BleedSystem;
mod particle_system;

#[derive(PartialEq, Clone, Copy)]
pub enum RunState { 
    AwaitingInput,
    PreRun,
    PlayerTurn,
    GameworldTurn,
    ShowPlayerMenu { menu_state: gui::PlayerMenuState },
    ShowContextMenu { selection: i8, focus: i8 },
    ShowTargeting { range: i32, item: Entity },
    MainMenu { menu_selection: gui::MainMenuSelection },
    SaveGame,
    NextLevel,
    GameOver,
}

pub struct State {
    pub ecs: World,
    pub tooltips_on: bool,
}

impl State {
    fn run_systems(&mut self) {
        let mut context_menu = ContextMenuSystem{};
        context_menu.run_now(&self.ecs);
        let mut mob = HostileAI{};
        mob.run_now(&self.ecs);
        let mut items = ItemUseSystem{};
        items.run_now(&self.ecs);
        let mut drop = ItemDropSystem{};
        drop.run_now(&self.ecs);
        let mut melee = MeleeCombatSystem{};
        melee.run_now(&self.ecs);
        let mut vis = VisibilitySystem{};
        vis.run_now(&self.ecs);
        let mut mapindex = MapIndexingSystem{};
        mapindex.run_now(&self.ecs);
        let mut healing = HealingSystem{};
        healing.run_now(&self.ecs);
        let mut bleed = BleedSystem{};
        bleed.run_now(&self.ecs);
        let mut damage = DamageSystem{};
        damage.run_now(&self.ecs);
        let mut pick_up = ItemCollectionSystem{};
        pick_up.run_now(&self.ecs);
        let mut equips = EquipSystem{};
        equips.run_now(&self.ecs);
        let mut particles = particle_system::ParticleSpawnSystem{};
        particles.run_now(&self.ecs);

        self.ecs.maintain();
    }

    fn entities_to_remove_on_level_change(&mut self) -> Vec<Entity> {
        let entities = self.ecs.entities();
        let player = self.ecs.read_storage::<Player>();
        let backpack = self.ecs.read_storage::<InBackpack>();
        let equipped = self.ecs.read_storage::<Equipped>();

        let mut to_delete: Vec<Entity> = Vec::new();
        for (ent, (), (), ()) in (&entities, !&player, !&backpack, !&equipped).join() {
            to_delete.push(ent);
        }

        to_delete
    }

    fn goto_next_level(&mut self) {
        // Delete entities that aren't the player or his/her equipment
        let to_delete = self.entities_to_remove_on_level_change();
        for target in to_delete {
            self.ecs.delete_entity(target).expect("Unable to delete entity");
        }

        // Build a new map and place the player
        let worldmap;
        let curr_depth;
        {
            let mut worldmap_resource = self.ecs.write_resource::<Map>();
            curr_depth = worldmap_resource.depth;
            *worldmap_resource = Map::new_map_rooms_and_corridors(curr_depth + 1);
            worldmap = worldmap_resource.clone();
        }

        // Spawn bad guys
        for room in worldmap.rooms.iter().skip(1) {
            spawner::spawn_room(&mut self.ecs, room, curr_depth + 1);
        }

        // Place the player and update resources
        let (player_x, player_y) = worldmap.rooms[0].center();
        let mut player_position = self.ecs.write_resource::<Point>();
        *player_position = Point::new(player_x, player_y);
        let mut position_components = self.ecs.write_storage::<Position>();
        let player_entity = self.ecs.fetch::<Entity>();
        let player_pos_comp = position_components.get_mut(*player_entity);
        if let Some(player_pos_comp) = player_pos_comp {
            player_pos_comp.x = player_x;
            player_pos_comp.y = player_y;
        }

        // Mark the player's visibility as dirty
        let mut viewshed_components = self.ecs.write_storage::<Viewshed>();
        let vs = viewshed_components.get_mut(*player_entity);
        if let Some(vs) = vs {
            vs.dirty = true;
        }        

        // Notify the player and give them some health
        let mut gamelog = self.ecs.fetch_mut::<gamelog::GameLog>();
        gamelog.entries.push("You descend to the next level and take a moment to rest.".to_string());
        let mut stats_storage = self.ecs.write_storage::<Stats>();
        let player_stats = stats_storage.get_mut(*player_entity);
        if let Some(stats) = player_stats {
            let new_hp = f32::floor((stats.max_hp - stats.hp) as f32 / 2.0) as i32 + stats.hp;
            stats.hp = new_hp;
        }
    }
    fn game_over_cleanup(&mut self) {
        //Delete All Entities
        let mut to_delete = Vec::new();
        for e in self.ecs.entities().join() {
            to_delete.push(e);
        }
        for del in to_delete.iter() {
            self.ecs.delete_entity(*del).expect("Deletion failed");
        }

        // Build a new map and place the player
        let worldmap;
        {
            let mut worldmap_resource = self.ecs.write_resource::<Map>();
            *worldmap_resource = Map::new_map_rooms_and_corridors(1);
            worldmap = worldmap_resource.clone();
        }

        // Spawn bad guys
        for room in worldmap.rooms.iter().skip(1) {
            spawner::spawn_room(&mut self.ecs, room, 1);
        }

        // Place the player and update resources
        let (player_x, player_y) = worldmap.rooms[0].center();
        let player_entity = spawner::player(&mut self.ecs, player_x, player_y);
        let mut player_position = self.ecs.write_resource::<Point>();
        *player_position = Point::new(player_x, player_y);
        let mut position_components = self.ecs.write_storage::<Position>();
        let mut player_entity_writer = self.ecs.write_resource::<Entity>();
        *player_entity_writer = player_entity;
        let player_pos_comp = position_components.get_mut(player_entity);
        if let Some(player_pos_comp) = player_pos_comp {
            player_pos_comp.x = player_x;
            player_pos_comp.y = player_y;
        }

        // Mark the player's visibility as dirty
        let mut viewshed_components = self.ecs.write_storage::<Viewshed>();
        let vs = viewshed_components.get_mut(player_entity);
        if let Some(vs) = vs {
            vs.dirty = true;
        }                                               
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) { 
        let mut newrunstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            newrunstate = *runstate;
           
            //reset cursor to inactive state
            match *runstate {
                RunState::AwaitingInput => {}
                RunState::ShowTargeting {range: _, item: _} => {}
                _ => {
                    let mut cursor = self.ecs.fetch_mut::<Cursor>();
                    cursor.active = false;
                }
            }
        }

        ctx.cls(); //clearscreen

        particle_system::cull_dead_particles(&mut self.ecs, ctx);

        match newrunstate {
            RunState::MainMenu{..} => {}
            RunState::GameOver => {}
            _ => {
                draw_map(&self.ecs, ctx);
                
                {
                    let positions = self.ecs.read_storage::<Position>();
                    let renderables = self.ecs.read_storage::<Renderable>();
                    let map = self.ecs.fetch::<Map>();
               
                    //gather & sort render data before rendering so gui layering is proper
                    let mut render_data = (&positions, &renderables).join().collect::<Vec<_>>();
                    render_data.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));
                    for (pos, render) in render_data.iter() {
                        let idx = map.xy_idx(pos.x, pos.y);
                        if map.visible_tiles[idx] {ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph)}
                    }
                    
                    match ctx.key {
                        None => {}
                        Some(key) => match key {
                            VirtualKeyCode::T =>
                                self.tooltips_on = !self.tooltips_on,
                            _ => {}
                        }
                    }

                    gui::draw_ui(&self.ecs, ctx, self.tooltips_on);
                }
            }
        }

        match newrunstate {
            RunState::PreRun => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                newrunstate = player_input(&mut self.ecs, ctx);
            }
            RunState::PlayerTurn => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::GameworldTurn;
            }
            RunState::GameworldTurn => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::ShowContextMenu { selection, focus } => {
                let result = gui::open_context_menu(&self.ecs, ctx, selection, focus);

                match result.0 {
                    gui::MenuResult::Continue => {
                        newrunstate = RunState::ShowContextMenu { selection: result.2, focus: result.3 };
                    }
                    gui::MenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::MenuResult::Selected => {
                        let player = self.ecs.fetch::<Entity>();
                        let ranged_storage = self.ecs.read_storage::<Ranged>();
                        let ranged_item = ranged_storage.get((result.1).unwrap().1);

                        match result.1 {
                            None => {}
                            Some( (menu_option, chosen_ent) ) => match (menu_option, chosen_ent) {
                                (MenuOption::PickUp, _) => {
                                    let mut storage = self.ecs.write_storage::<PickUpIntent>();
                                    storage.insert(*player,
                                        PickUpIntent { item: chosen_ent, desired_by: *player })
                                        .expect("Unable to insert PickUpIntent.");
                                    newrunstate = RunState::PlayerTurn;
                                }
                                (MenuOption::Use, _) => {
                                    if let Some(r) = ranged_item {
                                        newrunstate =
                                            RunState::ShowTargeting { range: r.range, item: chosen_ent };
                                    } else {
                                        let mut storage = self.ecs.write_storage::<UseItemIntent>();
                                        storage.insert(*player,
                                            UseItemIntent { item: chosen_ent, target: None })
                                            .expect("Unable to insert UseItemIntent.");
                                        newrunstate = RunState::PlayerTurn;
                                    }
                                }
                                (MenuOption::DropIt, _) => {
                                    let mut storage = self.ecs.write_storage::<DropItemIntent>();
                                    storage.insert(*player,
                                        DropItemIntent { item: chosen_ent })
                                        .expect("Unable to insert DropItemIntent.");
                                    newrunstate = RunState::PlayerTurn;
                                }
                                (MenuOption::Equip, _) => {
                                    let mut storage = self.ecs.write_storage::<EquipIntent>();
                                    storage.insert(*player,
                                        EquipIntent { item: chosen_ent })
                                        .expect("Unable to insert EquipIntent.");
                                    newrunstate = RunState::PlayerTurn;
                                }
                                (MenuOption::Attack, _) => {
                                    let mut storage = self.ecs.write_storage::<MeleeIntent>();
                                    storage.insert(*player,
                                        MeleeIntent { target: chosen_ent })
                                        .expect("Unable to insert MeleeIntent.");
                                    newrunstate = RunState::PlayerTurn;
                                }
                            }
                        }
                    }
                }
            }
            RunState::ShowPlayerMenu { menu_state } => {
                let out = gui::open_player_menu(&self.ecs, ctx, menu_state);

                match out.mr {
                    gui::MenuResult::Continue => {
                        newrunstate = RunState::ShowPlayerMenu { menu_state: out } }
                    gui::MenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::MenuResult::Selected => {
                        let player = self.ecs.fetch::<Entity>();
                        let ranged_storage = self.ecs.read_storage::<Ranged>();
                        let ranged_item = ranged_storage.get(out.result.unwrap().1);

                        match out.result {
                            None => {}
                            Some( (menu_option, chosen_ent) ) => match (menu_option, chosen_ent) {
                                (MenuOption::PickUp, _) => {
                                    let mut storage = self.ecs.write_storage::<PickUpIntent>();
                                    storage.insert(*player,
                                        PickUpIntent { item: chosen_ent, desired_by: *player })
                                        .expect("Unable to insert PickUpIntent.");
                                    newrunstate = RunState::PlayerTurn;
                                }
                                (MenuOption::Use, _) => {
                                    if let Some(r) = ranged_item {
                                        newrunstate =
                                            RunState::ShowTargeting { range: r.range, item: chosen_ent };
                                    } else {
                                        let mut storage = self.ecs.write_storage::<UseItemIntent>();
                                        storage.insert(*player,
                                            UseItemIntent { item: chosen_ent, target: None })
                                            .expect("Unable to insert UseItemIntent.");
                                        newrunstate = RunState::PlayerTurn;
                                    }
                                }
                                (MenuOption::DropIt, _) => {
                                    let mut storage = self.ecs.write_storage::<DropItemIntent>();
                                    storage.insert(*player,
                                        DropItemIntent { item: chosen_ent })
                                        .expect("Unable to insert DropItemIntent.");
                                    newrunstate = RunState::PlayerTurn;
                                }
                                (MenuOption::Equip, _) => {
                                    let mut storage = self.ecs.write_storage::<EquipIntent>();
                                    storage.insert(*player,
                                        EquipIntent { item: chosen_ent })
                                        .expect("Unable to insert EquipIntent.");
                                    newrunstate = RunState::PlayerTurn;
                                }
                                (MenuOption::Attack, _) => {
                                    let mut storage = self.ecs.write_storage::<MeleeIntent>();
                                    storage.insert(*player,
                                        MeleeIntent { target: chosen_ent })
                                        .expect("Unable to insert MeleeIntent.");
                                    newrunstate = RunState::PlayerTurn;
                                }
                            }
                        }
                    }
                }
            }
            RunState::ShowTargeting {range, item} => {
                let result = gui::target_selection_mode(&mut self.ecs, ctx, range);
                match result.0 {
                    gui::MenuResult::Continue => {}
                    gui::MenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::MenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<UseItemIntent>();
                        intent.insert(*self.ecs.fetch::<Entity>(), UseItemIntent {item, target: result.1})
                            .expect("Unable to insert UseItemIntent.");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::NextLevel => {
                self.goto_next_level();
                newrunstate = RunState::PreRun;
            }
            RunState::SaveGame => {
                saveload_system::save_game(&mut self.ecs); 
                newrunstate = RunState::MainMenu {menu_selection: gui::MainMenuSelection::LoadGame};
            }
            RunState::MainMenu{..} => {
                let result = gui::main_menu(self, ctx);
                match result {
                    gui::MainMenuResult::NoSelection {selected} =>
                        newrunstate = RunState::MainMenu {menu_selection: selected},
                    gui::MainMenuResult::Selected {selected} => {
                        match selected {
                            gui::MainMenuSelection::NewGame => newrunstate = RunState::PreRun,
                            gui::MainMenuSelection::LoadGame => {
                                saveload_system::load_game(&mut self.ecs);
                                newrunstate = RunState::AwaitingInput;
                                saveload_system::delete_save();
                            }
                            gui::MainMenuSelection::Quit => {::std::process::exit(0);}
                        }
                    }
                }
            }
            RunState::GameOver => {
                let result = gui::game_over(ctx);
                match result {
                    gui::MenuResult::Continue => {}
                    gui::MenuResult::Cancel => {}
                    gui::MenuResult::Selected => {
                        self.game_over_cleanup();
                        newrunstate = RunState::MainMenu { menu_selection: gui::MainMenuSelection::NewGame };
                    }
                }
            }
        }


        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = newrunstate;
        }

        damage_system::delete_the_dead(&mut self.ecs);
    }
}

struct Cursor { 
    pub x: i32, 
    pub y: i32,
    pub active: bool
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let mut context = RltkBuilder::simple80x50()
        .with_title("Wizard of the Old Tongue")
        .build()?;

    context.with_post_scanlines(true);

    let mut gs = State {
        ecs: World::new(),
        tooltips_on: false,
    };

    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<Hostile>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<Stats>();
    gs.ecs.register::<MeleeIntent>();
    gs.ecs.register::<DamageOnUse>();
    gs.ecs.register::<DamageQueue>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<PickUpIntent>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<UseItemIntent>();
    gs.ecs.register::<DropItemIntent>();
    gs.ecs.register::<Consumable>();
    gs.ecs.register::<Ranged>();
    gs.ecs.register::<AoE>();
    gs.ecs.register::<Confusion>();
    gs.ecs.register::<Equippable>();
    gs.ecs.register::<Equipped>();
    gs.ecs.register::<Resistances>();
    gs.ecs.register::<Weapon>();
    gs.ecs.register::<EquipIntent>();
    gs.ecs.register::<UnequipIntent>();
    gs.ecs.register::<BasicAttack>();
    gs.ecs.register::<BlocksAttacks>();
    gs.ecs.register::<Menuable>();
    gs.ecs.register::<Creature>();
    gs.ecs.register::<Bleeding>();
    gs.ecs.register::<Healing>();
    gs.ecs.register::<Heals>();
    gs.ecs.register::<Immunities>();
    gs.ecs.register::<Particle>();
    gs.ecs.register::<SimpleMarker<SerializeMe>>();
    gs.ecs.register::<SerializationHelper>();

    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());
    gs.ecs.insert(rltk::RandomNumberGenerator::new());

    let map : Map = Map::new_map_rooms_and_corridors(1);
    let (player_x, player_y) = map.rooms[0].center(); 
    let player_entity = spawner::player(&mut gs.ecs, player_x, player_y);

    for room in map.rooms.iter().skip(1) {
        spawner::spawn_room(&mut gs.ecs, room, 1);
    }

    gs.ecs.insert(RunState::PreRun);
    gs.ecs.insert(particle_system::ParticleBuilder::new());
    gs.ecs.insert(map);
    gs.ecs.insert(player_entity);
    gs.ecs.insert(Point::new(player_x, player_y));
    gs.ecs.insert(gamelog::GameLog {
        entries: vec!["The Wandering Wood Moves With One's Peripheral Gaze".to_string()]});
    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());
 
    gs.ecs.insert(Cursor { x: player_x, y: player_y, active: false });

    rltk::main_loop(context, gs)
}


