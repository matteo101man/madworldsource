//! The in-game radar, cIdRadar (research/GRAB-THROW-MINIMAP.md part 2): core_us program 36 (the frame, centred at
//! (82, 290) on 640x360, 82x82) over the room map, the player arrow at the centre and a dot per enemy.
//! - The map (program 36 widget 50, kind 9: an 80x80 viewport on the room IMT): the floor's page of the c201 map (the
//!   pause map's `assets/native/ui/c201-map`), centred on the player and turned with the camera (radar up = camera
//!   forward); the page is a black floor plan on transparency over the frame's white disc. The native viewport clip is not
//!   translated, so the port composites the circle on the CPU each frame.
//! - Arrow (UV group 25 #6): turned by the player's yaw off the camera's.
//! - Enemy dots (addPoint 80225A48, type 3; UV group 25 #3): 1.5 px per metre in the camera's frame, culled beyond 37 px
//!   and fading from 30 px. Every live street grunt and RinRin.
//! [unresolved there] the map scale (taken equal to the dots' 1.5 px/m), the dot colour table (enemies drawn red).
//! Hidden in the intro, the pause and while the fight HUD is off.
use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::{Extent3d, TextureDimension, TextureFormat}};
use super::pause::native_ui;

const CENTER: Vec2 = Vec2::new(82., 290.);
const MAP_SIZE: f32 = 80.;
/// Output resolution of the composited map (layout pixels x 2).
const MAP_PIXELS: u32 = 160;
const PX_PER_M: f32 = 1.5;
const DOT_CULL: f32 = 37.;
const DOT_FADE: f32 = 30.;

#[derive(Resource)]
pub struct Radar { frame: mw_sim::ui::UiScene, map: Option<mw_data::ui::StageMap>, pages: Vec<Handle<Image>>, out: Handle<Image>,
                   arrow: (String, Rect), dot: (String, Rect) }
#[derive(Component)]
pub struct RadarNode;

pub fn setup(hud: Res<super::hud_ui::Hud>, assets: Res<AssetServer>, mut images: ResMut<Assets<Image>>, mut commands: Commands) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // the room map (tools/ui/stage_pause_map.py <room>): c201's for the street and RinRin's arena, c303's for Elise
    let map_file = format!("assets/native/ui/{}-map/map.json", super::boss_room().unwrap_or("c201"));
    let text = std::fs::read_to_string(root.join(map_file)).unwrap_or_default();
    let map = mw_data::ui::StageMap::parse(&text).ok();
    let layout = &hud.source().layout;
    let sprite = |group: u16, k: usize| -> Option<(String, Rect)> {
        let g = layout.groups.iter().find(|g| g.id == group)?;
        let (page, s) = layout.sprite(g.first + k)?;
        Some((page.image.clone(), Rect::new(s.rect[0] as f32, s.rect[1] as f32, s.rect[2] as f32, s.rect[3] as f32)))
    };
    let (Some(arrow), Some(dot)) = (sprite(25, 6), sprite(25, 3)) else { return; };
    let mut frame = hud.source().scene(36);
    frame.advance(30.);   // past its opening
    let pages = map.iter().flat_map(|m| m.pages.iter()).map(|p| assets.load(p.image.clone())).collect();
    let out = images.add(Image::new(Extent3d { width: MAP_PIXELS, height: MAP_PIXELS, depth_or_array_layers: 1 }, TextureDimension::D2,
                                    vec![0; (MAP_PIXELS * MAP_PIXELS * 4) as usize], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::all()));
    commands.insert_resource(Radar { frame, map, pages, out, arrow, dot });
}

pub fn present(radar: Option<Res<Radar>>, hud: Res<super::hud_ui::Hud>, play: Res<super::Play>, street: Res<super::street::Street>, intro: Res<super::intro::Intro>, baron: Res<super::baron::Baron>,
               pause: Res<super::pause::Pause>, assets: Res<AssetServer>, mut images: ResMut<Assets<Image>>, windows: Query<&Window>,
               nodes: Query<Entity, With<RadarNode>>, mut commands: Commands) {
    for e in &nodes { commands.entity(e).despawn(); }
    let Some(radar) = radar else { return; };
    // (the actor editor's menu takes the radar's corner)
    if intro.controls_blocked() || baron.cine.is_some() || pause.open || play.fight.player.hp <= 0 || super::editor::on() { return; }
    let Ok(window) = windows.single() else { return; };
    let viewport = Vec2::new(window.width(), window.height());
    let factor = (viewport.x / 640.).min(viewport.y / 360.);
    let offset = (viewport - Vec2::new(640., 360.) * factor) * 0.5;
    let node = |c: Vec2, size: Vec2| Node { position_type: PositionType::Absolute, left: px(offset.x + (c.x - size.x * 0.5) * factor),
                                            top: px(offset.y + (c.y - size.y * 0.5) * factor), width: px(size.x * factor), height: px(size.y * factor), ..default() };
    let pl = Vec3::from_array(play.fight.player.pos);
    let yaw = play.cam_yaw;
    let (fwd, right) = (Vec2::new(yaw.sin(), yaw.cos()), Vec2::new(-yaw.cos(), yaw.sin()));
    // the map: the floor's page sampled around the player in the camera's frame, cut to a soft circle
    let page = radar.map.as_ref().map(|m| radar.pages[m.layer(pl.y).min(radar.pages.len() - 1)].clone());
    if let (Some(map), Some(src)) = (radar.map.as_ref(), page.and_then(|page| images.get(&page).and_then(|im| im.data.clone().map(|d| (d, im.width() as i32, im.height() as i32))))) {
        let (data, w, h) = src;
        let (tp, ppu, o) = (map.texture_pixels, map.pixels_per_world_unit, map.origin);
        let mut buf = vec![0u8; (MAP_PIXELS * MAP_PIXELS * 4) as usize];
        let half = MAP_PIXELS as f32 * 0.5;
        let sx = w as f32 / tp; let sy = h as f32 / tp;
        for y in 0..MAP_PIXELS { for x in 0..MAP_PIXELS {
            let d = Vec2::new(x as f32 + 0.5 - half, y as f32 + 0.5 - half) / (MAP_PIXELS as f32 / MAP_SIZE);   // layout px from centre
            let r = d.length() / (MAP_SIZE * 0.5);
            if r > 1. { continue; }
            let world = Vec2::new(pl.x, pl.z) + right * (d.x / PX_PER_M) + fwd * (-d.y / PX_PER_M);
            let tx = ((tp * 0.5 + (world.x + o[0]) * ppu) * sx) as i32; let ty = ((tp * 0.5 + (world.y + o[1]) * ppu) * sy) as i32;
            let k = ((y * MAP_PIXELS + x) * 4) as usize;
            let edge = ((1. - r) / 0.06).clamp(0., 1.);
            // the page is the floor plan (black, white-edged) on transparency, drawn over the frame's white disc
            if tx < 0 || ty < 0 || tx >= w || ty >= h { continue; }
            let s = ((ty * w + tx) * 4) as usize;
            buf[k..k + 3].copy_from_slice(&data[s..s + 3]); buf[k + 3] = (data[s + 3] as f32 * edge) as u8;
        } }
        if let Some(mut out) = images.get_mut(&radar.out) { out.data = Some(buf); }
    }
    let mut map = ImageNode::new(radar.out.clone()); map.image_mode = NodeImageMode::Stretch;
    commands.spawn((map, node(CENTER, Vec2::splat(MAP_SIZE)), GlobalZIndex(615), RadarNode));
    // the frame (program 36)
    let layout = &hud.source().layout;
    for sprite in native_ui::sprites(&radar.frame, layout, layout, &assets, viewport, 600, None) { commands.spawn((sprite, RadarNode)); }
    // enemy dots: every live grunt, and RinRin
    let mut enemies: Vec<Vec3> = street.targets();
    if !play.fight.solo && !play.fight.victory && play.fight.rin.state != mw_sim::fight::rin_state::DEAD { enemies.push(Vec3::from_array(play.fight.rin.pos)); }
    if let Some((hp, _, at)) = play.boss { if hp > 0 { enemies.push(at); } }
    for e in enemies {
        let rel = Vec2::new(e.x - pl.x, e.z - pl.z);
        let p = Vec2::new(rel.dot(right), -rel.dot(fwd)) * PX_PER_M;
        let d = p.length();
        if d > DOT_CULL { continue; }
        let alpha = if d > DOT_FADE { 1. - (d - DOT_FADE) / 5. } else { 1. };
        let mut dot = ImageNode::new(assets.load(radar.dot.0.clone())); dot.rect = Some(radar.dot.1); dot.image_mode = NodeImageMode::Stretch;
        dot.color = Color::srgba(1., 0.18, 0.12, alpha.clamp(0., 1.));
        commands.spawn((dot, node(CENTER + p, Vec2::splat(6.)), GlobalZIndex(620), RadarNode));
    }
    // the player arrow at the centre, turned by the player's yaw off the camera's (native: 180 - player yaw + camera yaw
    // in its own yaw convention; here the arrow sprite points up at a zero turn)
    let mut arrow = ImageNode::new(assets.load(radar.arrow.0.clone())); arrow.rect = Some(radar.arrow.1); arrow.image_mode = NodeImageMode::Stretch;
    let turn = yaw - play.fight.player.loco.facing;
    commands.spawn((arrow, node(CENTER, Vec2::splat(14.)), UiTransform::from_rotation(Rot2::radians(turn)), GlobalZIndex(630), RadarNode));
}
