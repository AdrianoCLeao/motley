use std::collections::HashSet;

use bevy_ecs::entity::Entity;

#[derive(Default, Clone, Debug)]
pub struct Selection {
    primary: Option<Entity>,
    secondary: HashSet<Entity>,
}

impl Selection {
    pub fn select_single(&mut self, entity: Entity) {
        self.primary = Some(entity);
        self.secondary.clear();
    }

    pub fn deselect(&mut self) {
        self.primary = None;
        self.secondary.clear();
    }

    pub fn primary(&self) -> Option<Entity> {
        self.primary
    }

    pub fn has_selection(&self) -> bool {
        self.primary.is_some() || !self.secondary.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = Entity> + '_ {
        self.primary
            .iter()
            .copied()
            .chain(self.secondary.iter().copied())
    }
}
