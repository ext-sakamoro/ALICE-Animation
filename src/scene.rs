use alice_sdf::animation::{AnimatedSdf, Timeline};
use alice_sdf::SdfNode;
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Unique actor identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub u32);

/// Stack-allocated transform (36 bytes). SIMD-friendly layout.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(C, align(32))]
pub struct ActorTransform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for ActorTransform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl ActorTransform {
    /// Combine parent * child transforms.
    #[inline]
    pub fn combine(&self, child: &ActorTransform) -> ActorTransform {
        ActorTransform {
            position: self.position + self.rotation * (self.scale * child.position),
            rotation: self.rotation * child.rotation,
            scale: self.scale * child.scale,
        }
    }
}

/// A single actor in the scene (character, prop, effect, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub name: String,
    pub base_sdf: SdfNode,
    pub timeline: Option<Timeline>,
    pub local_transform: ActorTransform,
    pub parent: Option<ActorId>,
    pub visible: bool,
}

impl Actor {
    pub fn new(name: impl Into<String>, sdf: SdfNode) -> Self {
        Self {
            name: name.into(),
            base_sdf: sdf,
            timeline: None,
            local_transform: ActorTransform::default(),
            parent: None,
            visible: true,
        }
    }

    /// Set a keyframe timeline on this actor.
    pub fn with_timeline(mut self, timeline: Timeline) -> Self {
        self.timeline = Some(timeline);
        self
    }

    /// Set local transform.
    pub fn with_transform(mut self, transform: ActorTransform) -> Self {
        self.local_transform = transform;
        self
    }

    /// Set parent actor.
    pub fn with_parent(mut self, parent: ActorId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Evaluate this actor's SDF at a given time.
    /// If a timeline is set, produces an AnimatedSdf.evaluate_at() result.
    /// Otherwise returns the base SDF.
    #[inline]
    pub fn evaluate_sdf(&self, time: f32) -> SdfNode {
        match &self.timeline {
            Some(tl) => {
                let animated = AnimatedSdf::new(self.base_sdf.clone(), tl.clone());
                animated.evaluate_at(time)
            }
            None => self.base_sdf.clone(),
        }
    }
}

/// Scene graph managing all actors with parent-child hierarchy.
/// Vec-based storage: O(1) access by ActorId index (cache-friendly).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGraph {
    actors: Vec<Option<Actor>>,
    next_id: u32,
    pub root_actors: Vec<ActorId>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self {
            actors: Vec::new(),
            next_id: 0,
            root_actors: Vec::new(),
        }
    }

    /// Add an actor to the scene. Returns its unique ID.
    pub fn add_actor(&mut self, actor: Actor) -> ActorId {
        let id = ActorId(self.next_id);
        self.next_id += 1;
        if actor.parent.is_none() {
            self.root_actors.push(id);
        }
        let idx = id.0 as usize;
        if idx >= self.actors.len() {
            self.actors.resize_with(idx + 1, || None);
        }
        self.actors[idx] = Some(actor);
        id
    }

    /// Get an actor by ID. O(1) Vec index access.
    #[inline]
    pub fn get_actor(&self, id: ActorId) -> Option<&Actor> {
        self.actors.get(id.0 as usize).and_then(|a| a.as_ref())
    }

    /// Get a mutable reference to an actor. O(1).
    #[inline]
    pub fn get_actor_mut(&mut self, id: ActorId) -> Option<&mut Actor> {
        self.actors.get_mut(id.0 as usize).and_then(|a| a.as_mut())
    }

    /// Find an actor by name.
    pub fn find_by_name(&self, name: &str) -> Option<ActorId> {
        for (i, slot) in self.actors.iter().enumerate() {
            if let Some(a) = slot {
                if a.name == name {
                    return Some(ActorId(i as u32));
                }
            }
        }
        None
    }

    /// Compute world-space transform by walking up the parent chain.
    pub fn get_world_transform(&self, id: ActorId) -> ActorTransform {
        let actor = match self.get_actor(id) {
            Some(a) => a,
            None => return ActorTransform::default(),
        };
        match actor.parent {
            Some(parent_id) => {
                let parent_world = self.get_world_transform(parent_id);
                parent_world.combine(&actor.local_transform)
            }
            None => actor.local_transform,
        }
    }

    /// Get all actor IDs.
    pub fn actor_ids(&self) -> Vec<ActorId> {
        self.actors
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|_| ActorId(i as u32)))
            .collect()
    }

    /// Number of actors.
    #[inline]
    pub fn actor_count(&self) -> usize {
        self.actors.iter().filter(|s| s.is_some()).count()
    }

    /// Sum of all actor positions and count (for camera framing).
    #[inline]
    pub fn actor_positions_sum(&self) -> (Vec3, usize) {
        let mut sum = Vec3::ZERO;
        let mut count = 0usize;
        for actor in self.actors.iter().flatten() {
            if actor.visible {
                sum += actor.local_transform.position;
                count += 1;
            }
        }
        (sum, count)
    }

    /// Evaluate the entire scene at a given time, producing a union of all visible actor SDFs.
    pub fn evaluate_scene(&self, time: f32) -> SdfNode {
        let mut nodes: Vec<SdfNode> = Vec::with_capacity(self.actors.len());
        for slot in &self.actors {
            if let Some(actor) = slot {
                if !actor.visible {
                    continue;
                }
                nodes.push(actor.evaluate_sdf(time));
            }
        }
        match nodes.len() {
            0 => SdfNode::sphere(1.0), // fallback
            1 => nodes.into_iter().next().unwrap(),
            _ => {
                let mut result = nodes.remove(0);
                for node in nodes {
                    result = result.union(node);
                }
                result
            }
        }
    }
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_find_actor() {
        let mut sg = SceneGraph::new();
        let id = sg.add_actor(Actor::new("hero", SdfNode::sphere(1.0)));
        assert_eq!(sg.find_by_name("hero"), Some(id));
        assert_eq!(sg.actor_count(), 1);
    }

    #[test]
    fn test_parent_child_transform() {
        let mut sg = SceneGraph::new();
        let parent_id = sg.add_actor(Actor::new("parent", SdfNode::sphere(1.0)).with_transform(
            ActorTransform {
                position: Vec3::new(10.0, 0.0, 0.0),
                ..Default::default()
            },
        ));
        let child_id = sg.add_actor(
            Actor::new("child", SdfNode::sphere(0.5))
                .with_parent(parent_id)
                .with_transform(ActorTransform {
                    position: Vec3::new(0.0, 5.0, 0.0),
                    ..Default::default()
                }),
        );
        let world = sg.get_world_transform(child_id);
        assert!((world.position - Vec3::new(10.0, 5.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_evaluate_scene() {
        let mut sg = SceneGraph::new();
        sg.add_actor(Actor::new("a", SdfNode::sphere(1.0)));
        sg.add_actor(Actor::new("b", SdfNode::sphere(2.0)));
        let sdf = sg.evaluate_scene(0.0);
        // Should produce a Union tree (a union b)
        match &sdf {
            SdfNode::Union { a, b } => {
                // Both children should exist
                assert!(matches!(a.as_ref(), SdfNode::Sphere { .. }));
                assert!(matches!(b.as_ref(), SdfNode::Sphere { .. }));
            }
            _ => panic!("Expected Union"),
        }
    }
}
