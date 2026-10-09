//! The world map: themed regions, level nodes and the paths between them,
//! and which levels the player can reach given their progress.
//!
//! Reachability follows the rules of the old hub levels: the hero walks
//! freely through junctions and completed levels, and can step onto (and so
//! play) a level that isn't completed yet, but can't walk past it.

use std::collections::VecDeque;
use std::sync::LazyLock;

use enum_map::{Enum, EnumMap};

use crate::direction::Dir4;
use macroquad::math::{IVec2, Rect, Vec2, ivec2, vec2};
use serde::Deserialize;

use crate::levels::{self, Level};
use crate::progress::Progress;

/// The visual and musical theme of a region (and of the levels in it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Enum)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Theme {
    Cellar,
    Powder,
    Hall,
    Lab,
    Towers,
    Archive,
}

#[derive(Deserialize)]
struct MapFile {
    size: [i32; 2],
    regions: Vec<RegionDef>,
    corridors: Vec<CorridorDef>,
    nodes: Vec<NodeDef>,
    edges: Vec<EdgeDef>,
    props: Vec<PropDef>,
}

#[derive(Deserialize)]
struct RegionDef {
    theme: Theme,
    name: String,
    #[serde(default)]
    note: Option<String>,
    room: [i32; 4],
}

#[derive(Deserialize)]
struct CorridorDef {
    theme: Theme,
    rect: [i32; 4],
}

#[derive(Deserialize)]
struct NodeDef {
    id: String,
    #[serde(default)]
    level: Option<String>,
    pos: [f32; 2],
    /// Levels that must all be completed before this node opens.
    #[serde(default)]
    requires: Vec<String>,
}

/// `[from, to]` or `[from, to, [waypoints...]]`.
#[derive(Deserialize)]
#[serde(untagged)]
enum EdgeDef {
    Straight(String, String),
    Bent(String, String, Vec<[f32; 2]>),
}

#[derive(Deserialize)]
pub(crate) struct PropDef {
    pub(crate) sprite: crate::atlas::SpriteId,
    pos: [f32; 2],
    #[serde(default = "one")]
    pub(crate) scale: f32,
}

fn one() -> f32 {
    1.0
}

impl PropDef {
    pub(crate) fn pos(&self) -> Vec2 {
        cell_center(self.pos)
    }
}

/// An axis-aligned block of map cells.
#[derive(Clone, Copy)]
pub(crate) struct CellRect {
    pub(crate) min: IVec2,
    pub(crate) size: IVec2,
}

impl CellRect {
    fn from_def([x, y, w, h]: [i32; 4]) -> Self {
        Self {
            min: ivec2(x, y),
            size: ivec2(w, h),
        }
    }

    pub(crate) fn contains(self, cell: IVec2) -> bool {
        let rel = cell - self.min;
        rel.x >= 0 && rel.y >= 0 && rel.x < self.size.x && rel.y < self.size.y
    }

    pub(crate) fn rect(self) -> Rect {
        Rect::new(
            self.min.x as f32,
            self.min.y as f32,
            self.size.x as f32,
            self.size.y as f32,
        )
    }

    pub(crate) fn cells(self) -> impl Iterator<Item = IVec2> {
        (0..self.size.y).flat_map(move |y| (0..self.size.x).map(move |x| self.min + ivec2(x, y)))
    }
}

pub(crate) struct Region {
    pub(crate) theme: Theme,
    pub(crate) name: String,
    /// Shown when one of the region's levels is selected.
    pub(crate) note: Option<String>,
    pub(crate) room: CellRect,
}

pub(crate) struct Corridor {
    pub(crate) theme: Theme,
    pub(crate) rect: CellRect,
}

#[derive(Clone, Copy)]
pub(crate) enum NodeKind {
    Level(&'static Level),
    /// A path junction (plaza, doorway) with no level.
    Junction,
}

pub(crate) struct Node {
    pub(crate) kind: NodeKind,
    /// Center of the node in map cells.
    pub(crate) pos: Vec2,
    pub(crate) region: usize,
    /// Levels that must all be completed before this node opens.
    pub(crate) requires: Vec<&'static Level>,
}

impl Node {
    pub(crate) fn level(&self) -> Option<&'static Level> {
        match self.kind {
            NodeKind::Level(level) => Some(level),
            NodeKind::Junction => None,
        }
    }

    /// Whether the node's requirements are met (a completed level stays
    /// open regardless, for progress saved before a requirement existed).
    fn open(&self, progress: &Progress) -> bool {
        self.requires.iter().all(|l| progress.is_completed(l.name))
            || self.level().is_some_and(|l| progress.is_completed(l.name))
    }

    /// Whether the hero can walk through this node.
    fn passable(&self, progress: &Progress) -> bool {
        self.level()
            .is_none_or(|level| progress.is_completed(level.name))
    }
}

pub(crate) struct Edge {
    pub(crate) ends: [usize; 2],
    /// The path from `ends[0]` to `ends[1]`, endpoints included.
    pub(crate) points: Vec<Vec2>,
}

impl Edge {
    fn other(&self, node: usize) -> usize {
        if self.ends[0] == node {
            self.ends[1]
        } else {
            self.ends[0]
        }
    }

    /// The path's points, starting from `node`.
    pub(crate) fn points_from(&self, node: usize) -> Vec<Vec2> {
        let mut points = self.points.clone();
        if self.ends[0] != node {
            points.reverse();
        }
        points
    }
}

pub(crate) struct WorldMap {
    /// Width and height in cells.
    pub(crate) size: IVec2,
    pub(crate) regions: Vec<Region>,
    pub(crate) corridors: Vec<Corridor>,
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) props: Vec<PropDef>,
    /// Where the hero starts a new game.
    pub(crate) start: usize,
}

pub(crate) static WORLD_MAP: LazyLock<WorldMap> =
    LazyLock::new(|| WorldMap::parse(include_str!("../../levels/world_map.json")));

/// The center of the cell at the given cell coordinates.
fn cell_center([x, y]: [f32; 2]) -> Vec2 {
    vec2(x + 0.5, y + 0.5)
}

/// Try every way of giving the paths distinct directions, keeping the one
/// whose directions best match where the paths head.
fn assign_directions(
    paths: &[(usize, Vec2)],
    chosen: EnumMap<Dir4, Option<usize>>,
    score: f32,
    best: &mut (f32, EnumMap<Dir4, Option<usize>>),
) {
    let Some((&(next, heading), rest)) = paths.split_first() else {
        if score > best.0 {
            *best = (score, chosen);
        }
        return;
    };
    for dir in Dir4::all() {
        if chosen[dir].is_none() {
            let d = dir.delta();
            let mut with = chosen;
            with[dir] = Some(next);
            let fit = heading.dot(vec2(d.dx as f32, d.dy as f32));
            assign_directions(rest, with, score + fit, best);
        }
    }
}

impl WorldMap {
    fn parse(json: &str) -> Self {
        let file: MapFile = serde_json::from_str(json).expect("invalid world map JSON");
        let regions: Vec<Region> = file
            .regions
            .into_iter()
            .map(|r| Region {
                theme: r.theme,
                name: r.name,
                note: r.note,
                room: CellRect::from_def(r.room),
            })
            .collect();
        let nodes: Vec<Node> = file
            .nodes
            .iter()
            .map(|n| {
                let pos = cell_center(n.pos);
                let cell = pos.floor().as_ivec2();
                let region = regions
                    .iter()
                    .position(|r| r.room.contains(cell))
                    .unwrap_or_else(|| panic!("map node {} is outside every room", n.id));
                let kind = match &n.level {
                    Some(name) => NodeKind::Level(
                        levels::get_level(name)
                            .unwrap_or_else(|| panic!("map node {} has unknown level", n.id)),
                    ),
                    None => NodeKind::Junction,
                };
                let requires = n
                    .requires
                    .iter()
                    .map(|name| {
                        levels::get_level(name).unwrap_or_else(|| {
                            panic!("map node {} requires unknown level {name}", n.id)
                        })
                    })
                    .collect();
                Node {
                    kind,
                    pos,
                    region,
                    requires,
                }
            })
            .collect();
        let index = |id: &str| {
            file.nodes
                .iter()
                .position(|n| n.id == id)
                .unwrap_or_else(|| panic!("map edge references unknown node {id}"))
        };
        let edges = file
            .edges
            .iter()
            .map(|e| {
                let (a, b, waypoints) = match e {
                    EdgeDef::Straight(a, b) => (a, b, &[][..]),
                    EdgeDef::Bent(a, b, waypoints) => (a, b, &waypoints[..]),
                };
                let ends = [index(a), index(b)];
                let points = std::iter::once(nodes[ends[0]].pos)
                    .chain(waypoints.iter().map(|&p| cell_center(p)))
                    .chain(std::iter::once(nodes[ends[1]].pos))
                    .collect();
                Edge { ends, points }
            })
            .collect();
        let map = Self {
            size: IVec2::from(file.size),
            regions,
            corridors: file
                .corridors
                .into_iter()
                .map(|c| Corridor {
                    theme: c.theme,
                    rect: CellRect::from_def(c.rect),
                })
                .collect(),
            start: index("start"),
            nodes,
            edges,
            props: file.props,
        };
        for (node, n) in map.nodes.iter().enumerate() {
            map.exits(node);
            for level in &n.requires {
                assert!(
                    map.node_of_level(level.name).is_some(),
                    "map node {node} requires {}, which isn't on the map",
                    level.name
                );
            }
        }
        map
    }

    fn edges_at(&self, node: usize) -> impl Iterator<Item = &Edge> {
        self.edges.iter().filter(move |e| e.ends.contains(&node))
    }

    /// The edge joining two adjacent nodes.
    pub(crate) fn edge_between(&self, a: usize, b: usize) -> &Edge {
        self.edges_at(a)
            .find(|e| e.other(a) == b)
            .unwrap_or_else(|| panic!("map nodes {a} and {b} aren't adjacent"))
    }

    /// The chains still locking nodes: `[required level's node, locked
    /// node]` for each requirement not yet met.
    pub(crate) fn chains(&self, progress: &Progress) -> Vec<[usize; 2]> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| !node.open(progress))
            .flat_map(|(locked, node)| {
                node.requires
                    .iter()
                    .filter(|level| !progress.is_completed(level.name))
                    .map(move |level| {
                        let key = self
                            .node_of_level(level.name)
                            .expect("required levels are on the map");
                        [key, locked]
                    })
            })
            .collect()
    }

    /// Which nodes the hero can walk to.
    pub(crate) fn reachable(&self, progress: &Progress) -> Vec<bool> {
        let mut reached = vec![false; self.nodes.len()];
        reached[self.start] = true;
        let mut queue = VecDeque::from([self.start]);
        while let Some(node) = queue.pop_front() {
            for next in self.edges_at(node).map(|e| e.other(node)) {
                if !reached[next] && self.nodes[next].open(progress) {
                    reached[next] = true;
                    if self.nodes[next].passable(progress) {
                        queue.push_back(next);
                    }
                }
            }
        }
        reached
    }

    /// The nodes along the shortest walk between two reachable nodes,
    /// starting with `from` and ending with `to`. The walk only passes
    /// through nodes the hero can walk through.
    pub(crate) fn route(&self, from: usize, to: usize, progress: &Progress) -> Vec<usize> {
        let reached = self.reachable(progress);
        assert!(
            reached[from] && reached[to],
            "route between unreachable nodes"
        );
        let mut came_from: Vec<Option<usize>> = vec![None; self.nodes.len()];
        let mut queue = VecDeque::from([from]);
        while let Some(node) = queue.pop_front() {
            if node == to {
                break;
            }
            if node != from && !self.nodes[node].passable(progress) {
                continue;
            }
            for next in self.edges_at(node).map(|e| e.other(node)) {
                if next != from && reached[next] && came_from[next].is_none() {
                    came_from[next] = Some(node);
                    queue.push_back(next);
                }
            }
        }
        let mut route = vec![to];
        while let Some(&last) = route.last()
            && last != from
        {
            route.push(came_from[last].expect("reachable nodes are connected"));
        }
        route.reverse();
        route
    }

    /// Which way each path out of `node` goes: the assignment of
    /// directions to paths that best matches where they head.
    pub(crate) fn exits(&self, node: usize) -> EnumMap<Dir4, Option<usize>> {
        let paths: Vec<(usize, Vec2)> = self
            .edges_at(node)
            .map(|e| {
                let points = e.points_from(node);
                (e.other(node), (points[1] - points[0]).normalize())
            })
            .collect();
        assert!(paths.len() <= 4, "map node {node} has more than four paths");
        let mut best = (f32::NEG_INFINITY, EnumMap::default());
        assign_directions(&paths, EnumMap::default(), 0.0, &mut best);
        best.1
    }

    /// The nodes walked through by stepping from `node` in `dir`: to the
    /// next stop along that path, passing straight through junctions that
    /// lead only onward. None if there's no path that way or it's locked.
    pub(crate) fn step(&self, node: usize, dir: Dir4, reached: &[bool]) -> Option<Vec<usize>> {
        let mut route = vec![self.exits(node)[dir]?];
        loop {
            let at = *route.last().expect("route has a first step");
            if !reached[at] {
                return None;
            }
            let previous = route.len().checked_sub(2).map_or(node, |i| route[i]);
            let onward: Vec<usize> = self
                .edges_at(at)
                .map(|e| e.other(at))
                .filter(|&n| n != previous)
                .collect();
            match (self.nodes[at].kind, onward.as_slice()) {
                (NodeKind::Junction, &[next]) if at != self.start => route.push(next),
                _ => return Some(route),
            }
        }
    }

    pub(crate) fn node_of_level(&self, level: &str) -> Option<usize> {
        self.nodes
            .iter()
            .position(|n| n.level().is_some_and(|l| l.name == level))
    }

    /// The theme of the region a level is in (levels off the map use the
    /// cellar theme).
    pub(crate) fn theme_of(&self, level: &str) -> Theme {
        self.node_of_level(level)
            .map_or(Theme::Cellar, |n| self.regions[self.nodes[n].region].theme)
    }

    /// Whether any node in the region is reachable.
    pub(crate) fn region_revealed(&self, region: usize, reached: &[bool]) -> bool {
        self.nodes
            .iter()
            .zip(reached)
            .any(|(node, &r)| r && node.region == region)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reachable_levels(progress: &Progress) -> Vec<&'static str> {
        let map = &*WORLD_MAP;
        let mut names: Vec<_> = map
            .nodes
            .iter()
            .zip(map.reachable(progress))
            .filter(|&(_, r)| r)
            .filter_map(|(n, _)| Some(n.level()?.name))
            .collect();
        names.sort();
        names
    }

    #[test]
    fn new_game_can_only_play_the_first_level() {
        assert_eq!(reachable_levels(&Progress::default()), ["rats"]);
    }

    #[test]
    fn completing_a_level_opens_the_next() {
        let progress = Progress::with_completed(["rats"]);
        assert_eq!(reachable_levels(&progress), ["more_rats", "rats"]);
    }

    #[test]
    fn uncompleted_levels_block_the_path_beyond() {
        // Black Holes gates the rest of the world.
        let cellar = [
            "rats",
            "more_rats",
            "trapped_rat",
            "trapped_rat2_v2",
            "webs",
            "planks",
            "guidance",
        ];
        let progress = Progress::with_completed(cellar);
        assert!(!reachable_levels(&progress).contains(&"triggers"));
        let progress = Progress::with_completed(cellar.into_iter().chain(["blackhole_v2"]));
        let reachable = reachable_levels(&progress);
        for level in ["triggers", "explosives"] {
            assert!(reachable.contains(&level), "{level} should be open");
        }
        assert!(!reachable.contains(&"order_of_operations_new_v2"));
    }

    #[test]
    fn countdown_waits_for_the_trigger_and_explosive_levels() {
        let cellar = ["rats", "more_rats", "webs", "planks", "blackhole_v2"];
        let some = Progress::with_completed(cellar.into_iter().chain(["triggers", "explosives"]));
        assert!(!reachable_levels(&some).contains(&"triggering_explosives_v3"));
        let all = Progress::with_completed(cellar.into_iter().chain([
            "triggers",
            "explosives",
            "explosives2",
        ]));
        assert!(reachable_levels(&all).contains(&"triggering_explosives_v3"));
    }

    #[test]
    fn chains_lead_from_each_unmet_requirement_to_its_lock() {
        let map = &*WORLD_MAP;
        let node = |name| map.node_of_level(name).unwrap();
        let countdown = node("triggering_explosives_v3");
        let cellar = ["rats", "more_rats", "webs", "planks", "blackhole_v2"];
        let mut chains = map.chains(&Progress::with_completed(cellar));
        chains.sort();
        let mut expected = [
            [node("triggers"), countdown],
            [node("explosives"), countdown],
            [node("explosives2"), countdown],
        ];
        expected.sort();
        assert_eq!(chains, expected);
        let progress =
            Progress::with_completed(cellar.into_iter().chain(["triggers", "explosives"]));
        assert_eq!(map.chains(&progress), [[node("explosives2"), countdown]]);
        let progress = Progress::with_completed(cellar.into_iter().chain([
            "triggers",
            "explosives",
            "explosives2",
        ]));
        assert!(map.chains(&progress).is_empty());
    }

    #[test]
    fn great_hall_opens_every_wing() {
        let progress = Progress::with_completed([
            "rats",
            "more_rats",
            "webs",
            "planks",
            "blackhole_v2",
            "triggers",
            "explosives",
            "explosives2",
            "triggering_explosives_v3",
            "order_of_operations_new_v2",
        ]);
        let reachable = reachable_levels(&progress);
        for level in [
            "lock_in",
            "chase",
            "reload_v3",
            "limited2",
            "synchronicity",
            "cyborg_rats/ai_takeover",
            "cyborg_rats/stalemate",
            "cooperation/cooperation",
            "old_levels/overstep",
        ] {
            assert!(reachable.contains(&level), "{level} should be open");
        }
        assert!(!reachable.contains(&"release"));
        assert!(!reachable.contains(&"cooperation/blocked_v2"));
    }

    #[test]
    fn every_level_is_on_the_map_except_unfinished_ones() {
        let map = &*WORLD_MAP;
        for level in levels::all() {
            let on_map = map.node_of_level(level.name).is_some();
            let unfinished = level.name.starts_with("gimmicks/");
            assert_eq!(on_map, !unfinished, "{}", level.name);
        }
    }

    #[test]
    fn route_walks_through_completed_levels() {
        let map = &*WORLD_MAP;
        let progress = Progress::with_completed(["rats", "more_rats"]);
        let webs = map.node_of_level("webs").unwrap();
        let route = map.route(map.start, webs, &progress);
        let names: Vec<_> = route
            .iter()
            .map(|&n| map.nodes[n].level().map_or("junction", |l| l.name))
            .collect();
        assert_eq!(names, ["junction", "rats", "more_rats", "webs"]);
    }

    fn step_to(from: &str, dir: Dir4, progress: &Progress) -> Option<&'static str> {
        let map = &*WORLD_MAP;
        let reached = map.reachable(progress);
        let node = map.node_of_level(from).unwrap();
        let route = map.step(node, dir, &reached)?;
        Some(
            map.nodes[*route.last().unwrap()]
                .level()
                .map_or("junction", |l| l.name),
        )
    }

    #[test]
    fn each_direction_steps_one_level_along_its_path() {
        let progress = Progress::with_completed(["rats", "more_rats", "webs"]);
        let exits: Vec<_> = Dir4::all()
            .into_iter()
            .filter_map(|dir| step_to("more_rats", dir, &progress))
            .collect();
        let mut sorted = exits.clone();
        sorted.sort();
        assert_eq!(sorted, ["rats", "trapped_rat", "webs"]);
        // Webs leads back down to More Rats, not past it to Rats.
        let back = Dir4::all()
            .into_iter()
            .filter_map(|dir| step_to("webs", dir, &progress))
            .collect::<Vec<_>>();
        assert!(back.contains(&"more_rats") && !back.contains(&"rats"));
    }

    #[test]
    fn locked_levels_cannot_be_stepped_to() {
        let progress = Progress::with_completed(["rats"]);
        let reached: Vec<_> = Dir4::all()
            .into_iter()
            .filter_map(|dir| step_to("more_rats", dir, &progress))
            .collect();
        assert_eq!(reached, ["rats"]);
    }

    #[test]
    fn every_stop_has_at_most_one_path_per_direction() {
        let map = &*WORLD_MAP;
        for node in 0..map.nodes.len() {
            let exits = map.exits(node);
            let count = exits.values().flatten().count();
            assert_eq!(count, map.edges_at(node).count(), "node {node}");
        }
    }
}
