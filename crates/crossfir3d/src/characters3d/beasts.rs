//! Animal and monster clips, and binding a body's bones to its animation player.

use super::*;

/// Clip names (after any "Armature|" prefix) to stand and to move with, best first.
const IDLE_CLIPS: [&str; 4] = ["Idle", "Flying_Idle", "Idle_2", "Flying"];
const WALK_CLIPS: [&str; 5] = ["Walk", "Fast_Flying", "Flying", "Gallop", "Run"];

/// A beast file's graph of its idle and walk clips, once its glTF has loaded.
type BeastClips = (Handle<AnimationGraph>, AnimationNodeIndex, AnimationNodeIndex);

/// Plays each beast's idle or walk clip on its own animation player, and turns it to
/// face the way it moves.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn animate_beasts(
    mut commands: Commands,
    server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut files: Local<HashMap<String, (Handle<Gltf>, Option<BeastClips>)>>,
    mut beasts: Query<(Entity, &mut Beast, &mut Transform, Option<&Glide>)>,
    children: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer, Has<AnimationGraphHandle>)>,
) {
    for (root, mut beast, mut t, glide) in &mut beasts {
        let (gltf, clips) = files
            .entry(beast.file.clone())
            .or_insert_with(|| (server.load(format!("models/{}", beast.file)), None));
        if clips.is_none()
            && let Some(g) = gltfs.get(gltf.id())
        {
            // "Armature|Walk" and "TRex_Walk" both count as "Walk".
            let is = |name: &str, w: &str| {
                let name = name.rsplit('|').next().unwrap_or(name);
                name == w || name.strip_suffix(w).is_some_and(|rest| rest.ends_with('_'))
            };
            let find = |wanted: &[&str]| {
                wanted.iter().find_map(|w| g.named_animations.iter().find(|(n, _)| is(n, w)).map(|(_, c)| c.clone()))
            };
            // Some animals only stand: they move in their idle clip.
            let Some(idle) = find(&IDLE_CLIPS) else { continue };
            let walk = find(&WALK_CLIPS).unwrap_or_else(|| idle.clone());
            let (graph, nodes) = AnimationGraph::from_clips([idle, walk]);
            *clips = Some((graphs.add(graph), nodes[0], nodes[1]));
        }
        let Some((graph, idle, walk)) = clips.clone() else { continue };
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

#[allow(clippy::too_many_arguments)]
/// Gives every bone of a spawned body part the id the clips use (its name path from
/// `Armature`), driven by the character's animation player.
pub(super) fn bind_bones(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    parts: Query<&PartOf>,
    previews: Query<(), With<PreviewBody>>,
    children: Query<&Children>,
    names: Query<&Name>,
    skinned: Query<(&Mesh3d, &SkinnedMesh)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut heads: Local<HashMap<AssetId<Mesh>, Handle<Mesh>>>,
) {
    let Ok(PartOf(root)) = parts.get(ready.entity) else { return };
    // The free body is a whole body; under an outfit only its head may show.
    for e in children.iter_descendants(ready.entity) {
        let Ok((mesh, skin)) = skinned.get(e) else { continue };
        // Mesh parts are named after their material: "<mesh>.MI_Superhero_Male".
        if !names.get(e).is_ok_and(|n| n.as_str().contains("MI_Superhero")) {
            continue;
        }
        let head = match heads.get(&mesh.0.id()) {
            Some(h) => h.clone(),
            None => {
                let is_head: Vec<bool> =
                    skin.joints.iter().map(|j| names.get(*j).is_ok_and(|n| matches!(n.as_str(), "Head" | "neck_01"))).collect();
                let Some(head) = meshes.get(&mesh.0).and_then(|m| head_only(m, &is_head)) else { continue };
                let h = meshes.add(head);
                heads.insert(mesh.0.id(), h.clone());
                h
            }
        };
        commands.entity(e).try_insert(Mesh3d(head));
    }
    if previews.contains(*root) {
        for e in children.iter_descendants(ready.entity) {
            commands.entity(e).try_insert(RenderLayers::layer(PREVIEW_LAYER));
        }
    }
    let armature = children.iter_descendants(ready.entity).find(|e| names.get(*e).is_ok_and(|n| n.as_str() == "Armature"));
    let Some(armature) = armature else { return };
    let Ok(name) = names.get(armature) else { return };
    let mut stack = vec![(armature, vec![name.clone()])];
    while let Some((e, path)) = stack.pop() {
        commands.entity(e).try_insert((AnimationTargetId::from_names(path.iter()), AnimatedBy(*root)));
        for &c in children.get(e).into_iter().flatten() {
            if let Ok(n) = names.get(c) {
                stack.push((c, [path.clone(), vec![n.clone()]].concat()));
            }
        }
    }
}

fn head_only(mesh: &Mesh, is_head: &[bool]) -> Option<Mesh> {
    let Some(VertexAttributeValues::Uint16x4(joints)) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX) else { return None };
    let Some(VertexAttributeValues::Float32x4(weights)) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT) else { return None };
    let indices: Vec<u32> = mesh.indices()?.iter().map(|i| i as u32).collect();
    let mut head = mesh.clone();
    head.insert_indices(Indices::U32(head_triangles(&indices, joints, weights, is_head)));
    Some(head)
}
