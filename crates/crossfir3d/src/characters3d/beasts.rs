//! Animal and monster clips, and binding a body's bones to its animation player.

use super::*;

/// Clip names (after any "Armature|" prefix) to stand and to move with, best first.
const IDLE_CLIPS: [&str; 4] = ["Idle", "Flying_Idle", "Idle_2", "Flying"];
const WALK_CLIPS: [&str; 5] = ["Walk", "Fast_Flying", "Flying", "Gallop", "Run"];

/// A beast file's graph of its idle and walk clips, once its glTF has loaded.
type BeastClips = (Handle<AnimationGraph>, AnimationNodeIndex, AnimationNodeIndex);

/// A beast file's glTF handle and, once loaded, its clips.
type BeastFile = (Handle<Gltf>, Option<BeastClips>);

/// The asset stores and per-file cache that turn a beast's glTF into its clips.
#[derive(SystemParam)]
pub(super) struct BeastFiles<'w, 's> {
    server: Res<'w, AssetServer>,
    gltfs: Res<'w, Assets<Gltf>>,
    graphs: ResMut<'w, Assets<AnimationGraph>>,
    files: Local<'s, HashMap<String, BeastFile>>,
}

impl BeastFiles<'_, '_> {
    /// The clips graph for a beast file, loading the file and building the graph once it is ready.
    fn clips(&mut self, file: &str) -> Option<BeastClips> {
        let (gltf, clips) = self.files.entry(file.to_string()).or_insert_with(|| (self.server.load(format!("models/{file}")), None));
        if clips.is_none()
            && let Some(g) = self.gltfs.get(gltf.id())
        {
            let (idle, walk) = find_clips(g)?;
            let (graph, nodes) = AnimationGraph::from_clips([idle, walk]);
            *clips = Some((self.graphs.add(graph), nodes[0], nodes[1]));
        }
        clips.clone()
    }
}

/// A glTF's idle and walk clips; None if it has no idle.
fn find_clips(g: &Gltf) -> Option<(Handle<AnimationClip>, Handle<AnimationClip>)> {
    // "Armature|Walk" and "TRex_Walk" both count as "Walk".
    let is = |name: &str, w: &str| {
        let name = name.rsplit('|').next().unwrap_or(name);
        name == w || name.strip_suffix(w).is_some_and(|rest| rest.ends_with('_'))
    };
    let find = |wanted: &[&str]| {
        wanted.iter().find_map(|w| g.named_animations.iter().find(|(n, _)| is(n, w)).map(|(_, c)| c.clone()))
    };
    // Some animals only stand: they move in their idle clip.
    let idle = find(&IDLE_CLIPS)?;
    let walk = find(&WALK_CLIPS).unwrap_or_else(|| idle.clone());
    Some((idle, walk))
}

/// Plays each beast's idle or walk clip on its own animation player, and turns it to
/// face the way it moves.
pub(super) fn animate_beasts(
    mut commands: Commands,
    mut files: BeastFiles,
    mut beasts: Query<(Entity, &mut Beast, &mut Transform, Option<&Glide>)>,
    children: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer, Has<AnimationGraphHandle>)>,
) {
    for (root, mut beast, mut t, glide) in &mut beasts {
        let Some((graph, idle, walk)) = files.clips(&beast.file) else { continue };
        let Some(found) = children.iter_descendants(root).find(|e| players.contains(*e)) else { continue };
        let Ok((player_e, mut player, has_graph)) = players.get_mut(found) else { continue };
        if !has_graph {
            commands.entity(player_e).try_insert(AnimationGraphHandle(graph));
        }
        let step = glide.map_or(Vec3::ZERO, |g| g.target - t.translation);
        let walking = step.length() > 0.02;
        if walking {
            t.rotation = Quat::from_rotation_y(step.x.atan2(step.z));
        }
        if beast.walking != Some(walking) {
            beast.walking = Some(walking);
            player.stop_all();
            player.play(if walking { walk } else { idle }).repeat();
        }
    }
}

/// The queries and caches `bind_bones` works with.
#[derive(SystemParam)]
pub(super) struct BoneParams<'w, 's> {
    commands: Commands<'w, 's>,
    parts: Query<'w, 's, &'static PartOf>,
    previews: Query<'w, 's, (), With<PreviewBody>>,
    children: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
    skinned: Query<'w, 's, (&'static Mesh3d, &'static SkinnedMesh)>,
    meshes: ResMut<'w, Assets<Mesh>>,
    heads: Local<'s, HashMap<AssetId<Mesh>, Handle<Mesh>>>,
}

impl BoneParams<'_, '_> {
    /// Swaps each MI_Superhero skinned mesh under `body` for its head-only copy.
    fn show_heads_only(&mut self, body: Entity) {
        for e in self.children.iter_descendants(body) {
            let Ok((mesh, skin)) = self.skinned.get(e) else { continue };
            // Mesh parts are named after their material: "<mesh>.MI_Superhero_Male".
            if !self.names.get(e).is_ok_and(|n| n.as_str().contains("MI_Superhero")) {
                continue;
            }
            let Some(head) = head_mesh(&self.names, &mut self.meshes, &mut self.heads, mesh, skin) else { continue };
            self.commands.entity(e).try_insert(Mesh3d(head));
        }
    }

    /// Puts everything under `body` on the preview render layer.
    fn put_on_preview_layer(&mut self, body: Entity) {
        for e in self.children.iter_descendants(body) {
            self.commands.entity(e).try_insert(RenderLayers::layer(PREVIEW_LAYER));
        }
    }

    /// Finds the `Armature` under `body` and tags it and every named bone below it.
    fn tag_bones(&mut self, body: Entity, root: Entity) {
        let armature = self.children.iter_descendants(body).find(|e| self.names.get(*e).is_ok_and(|n| n.as_str() == "Armature"));
        let Some(armature) = armature else { return };
        let Ok(name) = self.names.get(armature) else { return };
        let mut stack = vec![(armature, vec![name.clone()])];
        while let Some((e, path)) = stack.pop() {
            self.commands.entity(e).try_insert((AnimationTargetId::from_names(path.iter()), AnimatedBy(root)));
            for &c in self.children.get(e).into_iter().flatten() {
                if let Ok(n) = self.names.get(c) {
                    stack.push((c, [path.clone(), vec![n.clone()]].concat()));
                }
            }
        }
    }
}

/// The cached head-only copy of a skinned mesh, built on first use.
fn head_mesh(
    names: &Query<&Name>,
    meshes: &mut Assets<Mesh>,
    heads: &mut HashMap<AssetId<Mesh>, Handle<Mesh>>,
    mesh: &Mesh3d,
    skin: &SkinnedMesh,
) -> Option<Handle<Mesh>> {
    if let Some(h) = heads.get(&mesh.0.id()) {
        return Some(h.clone());
    }
    let is_head: Vec<bool> = skin.joints.iter().map(|j| names.get(*j).is_ok_and(|n| matches!(n.as_str(), "Head" | "neck_01"))).collect();
    let h = meshes.add(meshes.get(&mesh.0).and_then(|m| head_only(m, &is_head))?);
    heads.insert(mesh.0.id(), h.clone());
    Some(h)
}

/// Gives every bone of a spawned body part the id the clips use (its name path from
/// `Armature`), driven by the character's animation player.
pub(super) fn bind_bones(ready: On<WorldInstanceReady>, mut p: BoneParams) {
    let Ok(PartOf(root)) = p.parts.get(ready.entity) else { return };
    let root = *root;
    // The free body is a whole body; under an outfit only its head may show.
    p.show_heads_only(ready.entity);
    if p.previews.contains(root) {
        p.put_on_preview_layer(ready.entity);
    }
    p.tag_bones(ready.entity, root);
}

fn head_only(mesh: &Mesh, is_head: &[bool]) -> Option<Mesh> {
    let Some(VertexAttributeValues::Uint16x4(joints)) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX) else { return None };
    let Some(VertexAttributeValues::Float32x4(weights)) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT) else { return None };
    let indices: Vec<u32> = mesh.indices()?.iter().map(|i| i as u32).collect();
    let mut head = mesh.clone();
    head.insert_indices(Indices::U32(head_triangles(&indices, joints, weights, is_head)));
    Some(head)
}
