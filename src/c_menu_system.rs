use specs::prelude::*;
use super::{Menuable, MenuOption, Item, Position, InBackpack, Equippable, Hostile};
pub struct ContextMenuSystem {}

impl<'a> System<'a> for ContextMenuSystem {
    type SystemData = ( Entities<'a>,
                        WriteStorage<'a, Menuable>,
                        ReadStorage<'a, Item>,
                        ReadStorage<'a, Position>,
                        ReadStorage<'a, Equippable>,
                        ReadStorage<'a, InBackpack>,
                        ReadStorage<'a, Hostile>,
                      );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut menuable,
            items, positions, equippable, in_backpack, hostile) = data;

        //Populate Menuable components.
        for (ent, menu) in (&entities, &mut menuable).join() {
            
            menu.options.clear();

            if let Some(_) = items.get(ent) {
                menu.options.push( (MenuOption::Use, "Use".to_string()) );
                
                if let Some(_) = positions.get(ent) {
                    menu.options.push( (MenuOption::PickUp, "Pick Up".to_string()) );
                }

                if let Some(_) = in_backpack.get(ent) {
                    menu.options.push( (MenuOption::DropIt, "Drop".to_string()) );
                }
                
                if let Some(_) = equippable.get(ent) {
                    menu.options.push( (MenuOption::Equip, "Equip".to_string()) );
                }
            }

            if let Some(_) = hostile.get(ent) { menu.options
                .push( (MenuOption::Attack, "Attack".to_string()) ); }

        }
    }
}
