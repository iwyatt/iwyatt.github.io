// formatted with the following command: 
// rustfmt rustcode.rs --edition 2024 --config max_width=50,array_width=50,blank_lines_upper_bound=0,chain_width=50,comment_width=50,fn_call_width=50,fn_params_layout="Compressed",fn_single_line=true,format_strings=true

use crate::npc::*;
use crate::resources::entities::{
    BlackFade, CharGFX, GPReady, GameTimer,
    LifeMeterBox, RustCode, TetrisGrid,
    TetrisGridPointState, TetrisPiece,
    WeaponBoxSelect, WilyProp,
};
use crate::resources::explosion_orb::*;
use crate::resources::hitflash::*;
use crate::resources::megaman::MegaMan;
use crate::resources::music::{
    MusicTrack, TrackName,
};
use crate::resources::sequencer::*;
use crate::resources::spaceship::{
    SpaceShip, SpaceShipAction,
};
use crate::*;
use macroquad::audio::set_sound_volume;
/// time in seconds that must elapse for a tetris piece to move down
const DROP_DOWN_SPEED: f32 = 0.75;
/// time in seconds that must elapse before repeating a drop down
const DROP_DOWN_CYCLE_TIME: f32 = 0.0125;
// See line 3023 in MM2PA.py
pub async fn run(
    gameworld: &mut GameWorld,
) -> GameState {
    // initialization
    if !gameworld.gamestate.is_initialized {
        initialize(gameworld).await;
        // we force a return after initialization which should affect a "settling frame" type
        //  behavior so that when we run the intro in the timer that it doesn't record an
        //  elapsed time from the recorded frame duration from the loading sequence
        return GameState {
            state: State::Gameplay,
            is_initialized: true,
        };
    }
    if gameworld
        .get_entities_by_type_and_property(
            |npc: &NPC| npc.npc_state,
            NPCState::Gameplay,
        )
        .first()
        .unwrap()
        .name
        == NPCName::DrWily
    {
        crate::gamestates::credits::handle_stars(
            gameworld,
        )
        .await;
    }
    if gameworld
        .get_entities_by_type_and_property(
            |npc: &NPC| npc.npc_state,
            NPCState::Gameplay,
        )
        .first()
        .unwrap()
        .name
        == NPCName::RustMan
    {
        // gameworld.get_entity_by_type::<entities::Credits>().position.y -= 0.5;
        gameworld.get_entity_by_type::
            <entities::RustCode>().position.y -= 0.5;
    }
    if gameworld
        .get_entities_by_type_and_property(
            |npc: &NPC| npc.npc_state,
            NPCState::Gameplay,
        )
        .first()
        .unwrap()
        .hitpoints
        > 0
    {
        manage_music(gameworld).await;
    }
    // intro sequence
    // if ready gfx still exists, then we are still in stage intro
    // TODO: change to using sequence manager
    let ready_gfx = gameworld.get_entities_by_type::<entities::GPReady>();
    if ready_gfx.first().is_some() {
        // if dr wily, run wily intro else, regular intro
        if gameworld
            .get_entities_by_type_and_property(
                |npc: &NPC| npc.npc_state,
                NPCState::Gameplay,
            )
            .first()
            .unwrap()
            .name
            == NPCName::DrWily
        {
            // if sequencer hasn't been created, create it
            if gameworld
                .get_entities_by_type_and_property(
                    |sequencer: &Sequencer| {
                        matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                    },
                    true,
                )
                .is_empty()
            {
                let sequencer = Sequencer {
                    sequence: SequenceName::DrWilyIntro(DrWilyIntro::Initialize),
                };
                register_entities!(gameworld, sequencer);
            }
            run_wily_intro(gameworld).await;
        } else {
            // start intro
            run_stage_intro(gameworld).await;
        }
        return GameState {
            state: State::Gameplay,
            is_initialized: true,
        };
    }
    // failure sequence
    if let Some(_) = gameworld
        .get_entities_by_type_and_property(
            |sequencer: &Sequencer| {
                matches!(
                    sequencer.sequence,
                    SequenceName::GamePlayDefeat(
                        _
                    )
                )
            },
            true,
        )
        .into_iter()
        .next()
    {
        return run_fail_sequence(gameworld)
            .await;
    }
    // victory sequence (if robot is exploding) - robot death
    if gameworld
        .get_entities_by_type_and_property(
            |npc: &NPC| npc.npc_state,
            NPCState::Gameplay,
        )
        .first_mut()
        .unwrap()
        .npc_action
        == NPCAction::Exploding
        || gameworld
            .get_entities_by_type_and_property(
                |npc: &NPC| npc.npc_state,
                NPCState::Gameplay,
            )
            .first_mut()
            .unwrap()
            .npc_action
            == NPCAction::WeaponInstalling
    {
        run_robot_defeated(gameworld).await;
        // if victory sequence complete for normal robots then
        if gameworld
            .get_entities_by_type_and_property(
                |npc: &NPC| npc.npc_state,
                NPCState::Gameplay,
            )
            .first_mut()
            .unwrap()
            .npc_action_frame_idx
            > 40
        {
            // clean up robot and carry over to stage select
            if let Some(robot) = gameworld
                .get_entities_by_type_and_property(|npc: &NPC| npc.npc_state, NPCState::Gameplay)
                .first_mut()
            {
                robot.gp_activated = true;

                return GameState {
                    state: State::StageSelect,
                    is_initialized: false,
                };
            }
        }
        // if victory sequence complete for dr wily then
        if gameworld
            .get_entities_by_type::<entities::GPBackground>()
            .first_mut()
            .unwrap()
            .alpha
            <= 0.
        {
            return GameState {
                state: State::Credits,
                is_initialized: false,
            };
        }
        return GameState {
            state: State::Gameplay,
            is_initialized: true,
        };
    }
    gameplay(gameworld).await;
    return GameState {
        state: State::Gameplay,
        is_initialized: true,
    };
}
async fn gameplay(gameworld: &mut GameWorld) {
    // gameplay
    // TODO: all the following:
    //  x capture and process keyboard events
    //  x move blocks (left, right, down)
    //  x collision detection for when blocks reach end or another block
    //  x clear completed rows, do robot damage. and move supported blocks down
    //  x implement space to fast drop tetris piece
    //  x implement other block types
    //  x rotate blocks
    //   x set center block to be bright
    //  x impl switching weapons
    //   x show active available weapons
    //   x keyboard inputs
    //   x warp out, in, change tileset
    //   x change location of weapon selection box
    //   x change from b&w graphic to active graphic
    //   x impl bonus damage
    //  x make mega man blink
    // Sequences
    //  x robot death
    //   x radiate death orbs 2 each in cardinal and 1 for each diaganol direction
    //   x when final death orb is outside screen bounds, play music, flash robot fade in from grey to color
    //   x warp mega man out
    //   x flash new robot weapon in
    //  x mm death
    //  x wily intro animation
    //  x wily death
    // Sub-States
    //  - pause / unpause
    //   - sequence: bat floating around
    //  - quit dialog
    //   - Sequence: warp in/out rush
    // ...
    // Check if there was a collision
    if let Some(tetris_piece) = gameworld
        .get_entities_by_type::<TetrisPiece>()
        .first_mut()
    {
        // we check the active piece's turn time elapsed so that we still have a moment to move left/right
        //  even when at bottom of tetris grid
        if tetris_piece.turn_time_elapsed
            >= DROP_DOWN_SPEED
        {
            if process_collisions(gameworld) {
                let active_weapon = gameworld
                    .get_entities_by_type::<MegaMan>()
                    .first()
                    .unwrap()
                    .active_weapon
                    .clone();
                match active_weapon {
                    Some(npc::NPCName::BubbleMan) => {
                        gameworld.sfx_atlas.play(sfx::SFXName::BubbleShot).await
                    }
                    Some(npc::NPCName::AirMan) => {
                        gameworld.sfx_atlas.play(sfx::SFXName::AirShot).await
                    }
                    Some(npc::NPCName::QuickMan) => {
                        gameworld.sfx_atlas.play(sfx::SFXName::QuickShot).await
                    }
                    Some(npc::NPCName::HeatMan) => {
                        gameworld.sfx_atlas.play(sfx::SFXName::HeatShot).await
                    }
                    Some(npc::NPCName::WoodMan) => {
                        gameworld.sfx_atlas.play(sfx::SFXName::WoodShot).await
                    }
                    Some(npc::NPCName::MetalMan) => {
                        gameworld.sfx_atlas.play(sfx::SFXName::MetalShot).await
                    }
                    Some(npc::NPCName::FlashMan) => {
                        gameworld.sfx_atlas.play(sfx::SFXName::FlashShot).await
                    }
                    Some(npc::NPCName::CrashMan) => {
                        gameworld.sfx_atlas.play(sfx::SFXName::CrashShot).await
                    }
                    _ => gameworld.sfx_atlas.play(sfx::SFXName::PShot).await,
                }
                gameworld
                    .get_entities_by_type::<TetrisGrid>()
                    .first_mut()
                    .unwrap()
                    .gfx_time_elapsed = 0.
            }
        }
    }
    // if there is not an active piece, insert one
    // we use the tetris_grid_timer variable to determine if enough time has passed after clearning lines to spawn new tetris piece
    let tetris_grid_timer = gameworld
        .get_entities_by_type::<TetrisGrid>()
        .first()
        .unwrap()
        .gfx_time_elapsed
        .clone();
    // TODO: I'm not sure the tetris_grid_timer is actually doing anything. It never gets reset anywhere.
    if gameworld
        .get_entities_by_type::<TetrisPiece>()
        .is_empty()
        && tetris_grid_timer > DROP_DOWN_SPEED
    {
        gameworld
            .get_entities_by_type::<TetrisGrid>()
            .first_mut()
            .unwrap()
            .gfx_time_elapsed = 0.;
        let mut new_tetris_piece =
            TetrisPiece::new();
        // check losing condition if new piece is overlapping existing grid points
        if is_piece_overlapping_grid_points(
            gameworld,
            &new_tetris_piece,
        ) {
            // if not overlapping an existing grid point, then lets set the graphics and insert the new piece
            let activated_weapon = gameworld
                .get_entities_by_type::<MegaMan>()
                .first()
                .unwrap()
                .active_weapon
                .clone();
            new_tetris_piece
                .set_piece_gfx(activated_weapon);
            register_entities!(
                gameworld,
                new_tetris_piece
            );
        } else {
            // set substate to failure sequence
            let fail_sequence_state = Sequencer {
                sequence:
                    SequenceName::GamePlayDefeat(
                        Defeat::Initialize,
                    ),
            };
            register_entities!(
                gameworld,
                fail_sequence_state
            );
            return;
        }
    }
    // Input Switch Weapon
    if key_pressed!(
        gameworld.input,
        KeyCode::F1,
        KeyCode::Key1,
        KeyCode::F2,
        KeyCode::Key2,
        KeyCode::F3,
        KeyCode::Key3,
        KeyCode::F4,
        KeyCode::Key4,
        KeyCode::F5,
        KeyCode::Key5,
        KeyCode::F6,
        KeyCode::Key6,
        KeyCode::F7,
        KeyCode::Key7,
        KeyCode::F8,
        KeyCode::Key8,
        KeyCode::F9,
        KeyCode::Key9
    ) {
        let last_input = gameworld
            .input
            .keys_down
            .iter()
            .next()
            .cloned();
        let weapon: Option<NPCName> =
            match last_input {
                Some(KeyCode::F1)
                | Some(KeyCode::Key1) => {
                    Some(NPCName::BubbleMan)
                }
                Some(KeyCode::F2)
                | Some(KeyCode::Key2) => {
                    Some(NPCName::AirMan)
                }
                Some(KeyCode::F3)
                | Some(KeyCode::Key3) => {
                    Some(NPCName::QuickMan)
                }
                Some(KeyCode::F4)
                | Some(KeyCode::Key4) => {
                    Some(NPCName::HeatMan)
                }
                Some(KeyCode::F5)
                | Some(KeyCode::Key5) => None,
                Some(KeyCode::F6)
                | Some(KeyCode::Key6) => {
                    Some(NPCName::WoodMan)
                }
                Some(KeyCode::F7)
                | Some(KeyCode::Key7) => {
                    Some(NPCName::MetalMan)
                }
                Some(KeyCode::F8)
                | Some(KeyCode::Key8) => {
                    Some(NPCName::FlashMan)
                }
                Some(KeyCode::F9)
                | Some(KeyCode::Key9) => {
                    Some(NPCName::CrashMan)
                }
                _ => None,
            };
        let currently_active_weapon = gameworld
            .get_entities_by_type::<MegaMan>()
            .first()
            .unwrap()
            .active_weapon
            .clone();
        if weapon != currently_active_weapon
            && (weapon.is_none()
                || gameworld
                    .get_entities_by_type::<NPC>()
                    .iter()
                    .filter(|npc| {
                        npc.is_defeated
                            && Some(npc.name)
                                == weapon
                    })
                    .count()
                    > 0)
        {
            switch_weapon(gameworld, weapon)
                .await;
        }
    }
    // TODO: move input to individual function(s)
    // Input Rotate Right (ClockWise)
    if (gameworld.input.contains(&KeyCode::Up) || gameworld.input.contains(&KeyCode::Kp8)
        || gameworld.input.contains(&KeyCode::Kp9) || gameworld.input.contains(&KeyCode::W)
        || gameworld.input.contains(&KeyCode::E))
        // TODO: update the tap gesture to detect taps within a region instead of a specific place
        || gameworld.input.get_tap_location().is_some()
    {
        let tetris_grid_reference = gameworld
            .get_entities_by_type::<TetrisGrid>()
            .first()
            .unwrap()
            .matrix
            .clone();
        if let Some(active_tetris_piece) =
            gameworld.get_entities_by_type::<TetrisPiece>().first_mut()
        {
            active_tetris_piece.rotate_clockwise(tetris_grid_reference);
        }
    }
    // Input Rotate Left
    if gameworld.input.contains(&KeyCode::Kp7)
        || gameworld.input.contains(&KeyCode::Q)
    {
        let tetris_grid_reference = gameworld
            .get_entities_by_type::<TetrisGrid>()
            .first()
            .unwrap()
            .matrix
            .clone();
        if let Some(active_tetris_piece) =
            gameworld.get_entities_by_type::<TetrisPiece>().first_mut()
        {
            active_tetris_piece.rotate_counter_clockwise(tetris_grid_reference);
        }
    }
    // Touch Move (left or right)
    // TODO: there is a lot of repeated code below that is duplicated with move left and move right input detection
    // Also, the bool tuple is basically indicating if one of "move left", "move right", or "neither" are recorded.
    // An enum would probably be more legible and idiomatic but seems like more code for limited improved
    // readability.
    let mut touch_move: (bool, bool) =
        (false, false);
    if gameworld.input.get_gestures().contains(
        &input_manager::Gesture::TapHold,
    ) {
        // if let Some(tap_pos) = gameworld.input.get_tap_location().clone() {
        if let Some(active_tetris_piece) =
            gameworld.get_entities_by_type::<TetrisPiece>().first_mut()
        {
            if let Some(center_block_gfx_pos_x) = active_tetris_piece
                .piece_type_shape
                .iter()
                .find(|gp| gp.point_state == TetrisGridPointState::Center)
                .map(|gp| gp.gfx_position.x)
            {
                // let tap_x = tap_pos.0;

                if let Some(last_touch) = touches().last() {
                    let tap_x = last_touch.position.x;
                    // move left
                    if center_block_gfx_pos_x > tap_x {
                        touch_move = (true, false);
                    }

                    // move right
                    if tap_x > center_block_gfx_pos_x {
                        touch_move = (false, true);
                    }
                }
            }
        }
        // }
    }
    // Input Move Left
    if (touch_move == (true, false)
        || gameworld
            .input
            .contains(&KeyCode::Left)
        || gameworld
            .input
            .contains(&KeyCode::Kp4)
        || gameworld.input.contains(&KeyCode::A))
        || gameworld
            .input
            .get_gestures()
            .contains(
            &input_manager::Gesture::SwipeLeft,
        )
    {
        if gameworld
            .entities
            .iter()
            .find(|e| {
                e.as_any().is::<TetrisPiece>()
            })
            .is_none()
        {
            return;
        };
        let active_tetris_piece_grid_points =
            gameworld
                .entities
                .iter()
                .find(|e| {
                    e.as_any().is::<TetrisPiece>()
                })
                .unwrap()
                .as_any()
                .downcast_ref::<TetrisPiece>()
                .unwrap()
                .piece_type_shape
                .clone();
        let left_most_grid_point =
            active_tetris_piece_grid_points
                .iter()
                .map(|gp| {
                    entities::idx_to_grid_point(
                        gp.point_idx,
                    )
                    .x
                })
                .min()
                .unwrap()
                .clone();
        // cant move left if active tetris piece is already at the far left
        if left_most_grid_point > 0 {
            let mut can_move = true;
            let tetris_grid = gameworld
                .entities
                .iter_mut()
                .find(|e| {
                    e.as_any().is::<TetrisGrid>()
                })
                .unwrap()
                .as_any_mut()
                .downcast_mut::<TetrisGrid>()
                .unwrap()
                .clone();
            // if _any_ grid point on the active tetris piece has a non-empty grid point to the left, then cannot move
            for gp in
                active_tetris_piece_grid_points
                    .iter()
            {
                let test_idx = gp.point_idx - 1;
                if tetris_grid.matrix
                    [test_idx as usize]
                    .point_state
                    != TetrisGridPointState::Empty
                {
                    can_move = false;
                }
            }
            if can_move {
                gameworld
                    .get_entities_by_type::<TetrisPiece>()
                    .first_mut()
                    .unwrap()
                    .move_left();
            }
        }
    };
    // Input Right - Move Piece Right
    if (touch_move == (false, true)
        || gameworld
            .input
            .contains(&KeyCode::Right)
        || gameworld
            .input
            .contains(&KeyCode::Kp6)
        || gameworld.input.contains(&KeyCode::D))
        || gameworld
            .input
            .get_gestures()
            .contains(
            &input_manager::Gesture::SwipeRight,
        )
    {
        if gameworld
            .entities
            .iter()
            .find(|e| {
                e.as_any().is::<TetrisPiece>()
            })
            .is_none()
        {
            return;
        };
        let active_tetris_piece_grid_points =
            gameworld
                .entities
                .iter()
                .find(|e| {
                    e.as_any().is::<TetrisPiece>()
                })
                .unwrap()
                .as_any()
                .downcast_ref::<TetrisPiece>()
                .unwrap()
                .piece_type_shape
                .clone();
        let right_most_grid_point =
            active_tetris_piece_grid_points
                .iter()
                .map(|gp| {
                    entities::idx_to_grid_point(
                        gp.point_idx,
                    )
                    .x
                })
                .max()
                .unwrap()
                .clone();
        // cant move right if active tetris piece is already at the far right
        if right_most_grid_point < 9 {
            let mut can_move = true;
            let tetris_grid = gameworld
                .entities
                .iter_mut()
                .find(|e| {
                    e.as_any().is::<TetrisGrid>()
                })
                .unwrap()
                .as_any_mut()
                .downcast_mut::<TetrisGrid>()
                .unwrap()
                .clone();
            // if _any_ grid point on the active tetris piece has a non-empty grid point to the left, then cannot move
            for gp in
                active_tetris_piece_grid_points
                    .iter()
            {
                let test_idx = gp.point_idx + 1;
                if tetris_grid.matrix
                    [test_idx as usize]
                    .point_state
                    != TetrisGridPointState::Empty
                {
                    can_move = false;
                }
            }
            if can_move {
                gameworld
                    .get_entities_by_type::<TetrisPiece>()
                    .first_mut()
                    .unwrap()
                    .move_right();
            }
        }
    }
    // Input Down - Move Piece Down
    if gameworld.input.contains(&KeyCode::Down)
        || gameworld.input.contains(&KeyCode::Kp2)
        || gameworld.input.contains(&KeyCode::Kp5)
        || gameworld.input.contains(&KeyCode::S)
        || gameworld.input.contains(&KeyCode::X)
    {
        let tetris_grid_matrix_reference = gameworld
            .get_entities_by_type::<TetrisGrid>()
            .first()
            .unwrap()
            .matrix
            .clone();
        if let Some(active_tetris_piece) =
            gameworld.get_entities_by_type::<TetrisPiece>().first_mut()
        {
            let lowest_grid_point = active_tetris_piece
                .piece_type_shape
                .iter()
                .map(|gp| entities::idx_to_grid_point(gp.point_idx).y)
                .max()
                .unwrap()
                .clone();
            // all grid points on the active tetris piece need to be eligible to move down on the tetris grid
            if lowest_grid_point < 13
                && active_tetris_piece.piece_type_shape.iter().all(|gp| {
                    tetris_grid_matrix_reference[gp.point_idx as usize + 10].point_state
                        == TetrisGridPointState::Empty
                })
            {
                for gp in active_tetris_piece.piece_type_shape.iter_mut() {
                    gp.point_idx += 10;
                    gp.update_gfx_pos();
                    // if you press down to accelerate the block, reset the drop down timer
                    active_tetris_piece.turn_time_elapsed = 0.;
                }
            }
        }
    }
    // Press Space to drop piece ultra fast
    if get_keys_down().contains(&KeyCode::Space)
        || get_keys_down().contains(&KeyCode::Kp0)
        || gameworld
            .input
            .get_gestures()
            .contains(&input_manager::Gesture::SwipeHoldDown)
    {
        if let Some(active_tetris_piece) =
            gameworld.get_entities_by_type::<TetrisPiece>().first_mut()
        {
            if active_tetris_piece.turn_time_elapsed >= DROP_DOWN_CYCLE_TIME {
                active_tetris_piece.turn_time_elapsed = DROP_DOWN_SPEED;
            }
        }
    }
    // No Input - set the new position of the active tetris piece
    let tetris_grid_matrix_reference = gameworld
        .get_entities_by_type::<TetrisGrid>()
        .first()
        .unwrap()
        .matrix
        .clone();
    if let Some(active_tetris_piece) = gameworld
        .get_entities_by_type::<TetrisPiece>()
        .first_mut()
    {
        if active_tetris_piece.turn_time_elapsed
            >= DROP_DOWN_SPEED
        {
            let lowest_grid_point = active_tetris_piece
                .piece_type_shape
                .iter()
                .map(|gp| entities::idx_to_grid_point(gp.point_idx).y)
                .max()
                .unwrap()
                .clone();
            if lowest_grid_point < 13
                && active_tetris_piece.piece_type_shape.iter().all(|gp| {
                    tetris_grid_matrix_reference[gp.point_idx as usize + 10].point_state
                        == TetrisGridPointState::Empty
                })
            {
                for gp in active_tetris_piece.piece_type_shape.iter_mut() {
                    gp.point_idx += 10;
                    gp.update_gfx_pos();
                }
                active_tetris_piece.turn_time_elapsed = 0.0;
            }
        }
    }
    clear_lines(gameworld).await;
}
async fn run_wily_intro(
    gameworld: &mut GameWorld,
) {
    let sequencer_step = gameworld
        .get_entities_by_type_and_property(
            |sequencer: &Sequencer| {
                matches!(
                    sequencer.sequence,
                    SequenceName::DrWilyIntro(_)
                )
            },
            true,
        )
        .first()
        .unwrap()
        .sequence
        .clone();
    match sequencer_step {
        SequenceName::DrWilyIntro(
            DrWilyIntro::Initialize,
        ) => {
            if let Some(timer) = gameworld
                .get_entities_by_type_and_property(
                    |timer: &entities::Timer| timer.name.clone(),
                    "intro_sequence".to_string(),
                )
                .first_mut()
            {
                if !timer.is_active {
                    timer.is_active = true;
                }
            }
            gameworld
                .get_entities_by_type_and_property(
                    |sequencer: &Sequencer| {
                        matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                    },
                    true,
                )
                .first_mut()
                .unwrap()
                .sequence = SequenceName::DrWilyIntro(DrWilyIntro::WaitForReadyTimer);
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::WaitForReadyTimer,
        ) => {
            let timer_elapsed = gameworld
                .get_entities_by_type_and_property(
                    |timer: &entities::Timer| timer.name.clone(),
                    "intro_sequence".to_string(),
                )
                .first()
                .unwrap()
                .elapsed_time;
            if timer_elapsed <= 3.0 {
                return;
            }
            if timer_elapsed > 3.0 {
                gameworld
                    .get_entities_by_type_and_property(
                        |sequencer: &Sequencer| {
                            matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                        },
                        true,
                    )
                    .first_mut()
                    .unwrap()
                    .sequence = SequenceName::DrWilyIntro(DrWilyIntro::WarpIn);
            }
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::WarpIn,
        ) => {
            if let Some(ready_gfx) = gameworld
                .get_entities_by_type::<entities::GPReady>()
                .first_mut()
            {
                ready_gfx.is_visible = false;
            }
            // step 2: warp in megaman
            let (megaman_action, megaman_position_y, megaman_frame_idx) = gameworld
                .get_entities_by_type::<megaman::MegaMan>()
                .first()
                .map(|megaman| (megaman.action, megaman.position.y, megaman.action_frame_idx))
                .unwrap();
            match megaman_action {
                megaman::Action::Nothing => {
                    gameworld
                        .get_entities_by_type::<megaman::MegaMan>()
                        .first_mut()
                        .unwrap()
                        .start_action(megaman::Action::WarpingIn);
                    gameworld
                        .sfx_atlas
                        .play(
                            sfx::SFXName::WarpIn,
                        )
                        .await;
                    return;
                }
                megaman::Action::WarpingIn => {
                    if megaman_position_y >= 18.
                        && megaman_frame_idx == 3
                    {
                        gameworld
                            .get_entities_by_type::<megaman::MegaMan>()
                            .first_mut()
                            .unwrap()
                            .start_action(megaman::Action::Standing);
                        return;
                    }
                }
                megaman::Action::Standing => {
                    gameworld
                        .sfx_atlas
                        .play_looped(
                        sfx::SFXName::SpaceShip,
                    );
                    gameworld
                        .get_entities_by_type_and_property(
                            |sequencer: &Sequencer| {
                                matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                            },
                            true,
                        )
                        .first_mut()
                        .unwrap()
                        .sequence = SequenceName::DrWilyIntro(DrWilyIntro::MoveShipAcross);
                }
                _ => return,
            }
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::MoveShipAcross,
        ) => {
            if let Some(spaceship) = gameworld.get_entities_by_type::<SpaceShip>().first_mut() {
                spaceship.is_active = true;
                spaceship.is_visible = true;

                if spaceship.action == SpaceShipAction::Idle {
                    spaceship.move_to(vec2(DSCREENSIZE.x + 40., spaceship.position.y), 45.0);
                }

                if spaceship.position.x >= DSCREENSIZE.x * 0.66 {
                    gameworld
                        .get_entities_by_type_and_property(
                            |sequencer: &Sequencer| {
                                matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                            },
                            true,
                        )
                        .first_mut()
                        .unwrap()
                        .sequence = SequenceName::DrWilyIntro(DrWilyIntro::SpawnWilyIn);
                }
            }
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::SpawnWilyIn,
        ) => {
            let spaceship_pos = gameworld
                .get_entities_by_type::<SpaceShip>()
                .first()
                .unwrap()
                .position
                .clone();
            let spaceship_gfx = gameworld
                .get_entities_by_type::<SpaceShip>()
                .first()
                .unwrap()
                .gfx_name;
            let spaceship_gfx_center = gameworld
                .loaded_textures
                .get(&spaceship_gfx)
                .unwrap()
                .frames
                .first()
                .unwrap()
                .size()
                / 2.;
            let center = vec2(
                spaceship_pos.x
                    + spaceship_gfx_center.x
                    - 20.,
                spaceship_pos.y
                    + spaceship_gfx_center.y
                    - 20.,
            );
            gameworld
                .get_entities_by_type::<WilyProp>(
                )
                .first_mut()
                .unwrap()
                .position = center;
            gameworld
                .get_entities_by_type::<WilyProp>(
                )
                .first_mut()
                .unwrap()
                .is_visible = true;
            gameworld
                .get_entities_by_type_and_property(
                    |sequencer: &Sequencer| {
                        matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                    },
                    true,
                )
                .first_mut()
                .unwrap()
                .sequence = SequenceName::DrWilyIntro(DrWilyIntro::WaitForShipOut);
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::WaitForShipOut,
        ) => {
            let spaceship_pos = gameworld
                .get_entities_by_type::<SpaceShip>()
                .first()
                .unwrap()
                .position
                .clone();
            if spaceship_pos.x
                >= DSCREENSIZE.x + 40.
            {
                gameworld
                    .get_entities_by_type_and_property(
                        |sequencer: &Sequencer| {
                            matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                        },
                        true,
                    )
                    .first_mut()
                    .unwrap()
                    .sequence = SequenceName::DrWilyIntro(DrWilyIntro::FloatWilyUp);
                gameworld.sfx_atlas.stop(
                    sfx::SFXName::SpaceShip,
                );
                gameworld.entities.retain(
                    |entity| {
                        !entity
                            .as_any()
                            .is::<SpaceShip>()
                    },
                );
            }
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::FloatWilyUp,
        ) => {
            gameworld
                .get_entities_by_type::<WilyProp>(
                )
                .first_mut()
                .unwrap()
                .is_active = true;
            gameworld
                .get_entities_by_type_and_property(
                    |sequencer: &Sequencer| {
                        matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                    },
                    true,
                )
                .first_mut()
                .unwrap()
                .sequence = SequenceName::DrWilyIntro(DrWilyIntro::WaitForWilyUp);
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::WaitForWilyUp,
        ) => {
            if gameworld
                .get_entities_by_type::<WilyProp>(
                )
                .first_mut()
                .unwrap()
                .position
                .y
                <= -48.
            {
                gameworld.entities.retain(
                    |entity| {
                        !entity
                            .as_any()
                            .is::<WilyProp>()
                    },
                );
                gameworld
                    .get_entities_by_type_and_property(
                        |sequencer: &Sequencer| {
                            matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                        },
                        true,
                    )
                    .first_mut()
                    .unwrap()
                    .sequence = SequenceName::DrWilyIntro(DrWilyIntro::FloatWilyDown);
            }
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::FloatWilyDown,
        ) => {
            if let Some(wily) = gameworld
                .get_entities_by_type_and_property(
                    |drwily: &NPC| drwily.npc_state,
                    NPCState::Gameplay,
                )
                .first_mut()
            {
                wily.move_to(vec2(74., 48.), 48.);

                gameworld
                    .get_entities_by_type_and_property(
                        |sequencer: &Sequencer| {
                            matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                        },
                        true,
                    )
                    .first_mut()
                    .unwrap()
                    .sequence = SequenceName::DrWilyIntro(DrWilyIntro::WaitForWilyDown);
            }
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::WaitForWilyDown,
        ) => {
            match gameworld
                .get_entities_by_type_and_property(
                    |drwily: &NPC| drwily.npc_state,
                    NPCState::Gameplay,
                )
                .first_mut()
                .unwrap()
                .npc_action
            {
                NPCAction::Intro => {
                    gameworld
                        .get_entities_by_type_and_property(
                            |sequencer: &Sequencer| {
                                matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                            },
                            true,
                        )
                        .first_mut()
                        .unwrap()
                        .sequence = SequenceName::DrWilyIntro(DrWilyIntro::MorphWily);
                }
                _ => {}
            }
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::MorphWily,
        ) => {
            if let Some(wily) = gameworld
                .get_entities_by_type_and_property(
                    |drwily: &NPC| drwily.npc_state,
                    NPCState::Gameplay,
                )
                .first_mut()
            {
                wily.start_action(NPCAction::Morphing);
            }
            gameworld
                .get_entities_by_type_and_property(
                    |sequencer: &Sequencer| {
                        matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                    },
                    true,
                )
                .first_mut()
                .unwrap()
                .sequence = SequenceName::DrWilyIntro(DrWilyIntro::WaitForMorphComplete);
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::WaitForMorphComplete,
        ) => {
            // fade out the intro music
            if let Some(track) = gameworld
                .get_entities_by_type_and_property(|wily12: &MusicTrack| wily12.is_playing, true)
                .first_mut()
            {
                let old_volume = track.params.volume.clone();
                let new_volume = old_volume - (get_frame_time() / 6.);
                if new_volume <= 0. {
                    track.stop();
                    track.is_playing = false;
                } else {
                    set_sound_volume(&track.track, new_volume);
                    track.params.volume = new_volume;
                }
            }
            // Fade in the starfield by increasing the alpha of the blackfade
            if let Some(blackfade) = gameworld.get_entities_by_type::<BlackFade>().first_mut() {
                blackfade.color.a -= 0.15 * get_frame_time();
            }
            match gameworld
                .get_entities_by_type_and_property(
                    |drwily: &NPC| drwily.npc_state,
                    NPCState::Gameplay,
                )
                .first_mut()
                .unwrap()
                .npc_action
            {
                // npcaction::intro will get set for wily when the the Move action is completed and is set in the entity update method of [NPC]
                NPCAction::Breathing => {
                    gameworld
                        .get_entities_by_type_and_property(
                            |sequencer: &Sequencer| {
                                matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                            },
                            true,
                        )
                        .first_mut()
                        .unwrap()
                        .sequence = SequenceName::DrWilyIntro(DrWilyIntro::FillLifeBar);
                }
                _ => {}
            }
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::FillLifeBar,
        ) => {
            // make life bar visible
            let (life_meter_box_is_visible, life_meter_box_qty, life_meter_box_frame_elapsed) =
                gameworld
                    .get_entities_by_type::<LifeMeterBox>()
                    .first_mut()
                    .map(|life_meter_box| {
                        (
                            life_meter_box.is_visible,
                            life_meter_box.qty,
                            life_meter_box.frame_elapsed,
                        )
                    })
                    .unwrap();
            if !life_meter_box_is_visible {
                gameworld
                    .get_entities_by_type::<LifeMeterBox>()
                    .first_mut()
                    .unwrap()
                    .is_visible = true;
                gameworld.sfx_atlas.play_looped(
                    sfx::SFXName::LifeMeterFill,
                );
            }
            if life_meter_box_frame_elapsed
                >= 0.09
                && life_meter_box_qty < 16
            {
                gameworld
                    .get_entities_by_type::<LifeMeterBox>()
                    .first_mut()
                    .unwrap()
                    .frame_elapsed = 0.;
                gameworld
                    .get_entities_by_type::<LifeMeterBox>()
                    .first_mut()
                    .unwrap()
                    .qty += 1;
            }
            if life_meter_box_qty == 16 {
                gameworld.sfx_atlas.stop(
                    sfx::SFXName::LifeMeterFill,
                );
                if let Some(track) = gameworld
                    .get_entities_by_type_and_property(
                        |music_track: &MusicTrack| music_track.name == TrackName::QGMWilyAltered,
                        true,
                    )
                    .first_mut()
                {
                    track.play();
                }
                gameworld
                    .get_entities_by_type_and_property(
                        |sequencer: &Sequencer| {
                            matches!(sequencer.sequence, SequenceName::DrWilyIntro(_))
                        },
                        true,
                    )
                    .first_mut()
                    .unwrap()
                    .sequence = SequenceName::DrWilyIntro(DrWilyIntro::EndSequence);
            }
        }

        SequenceName::DrWilyIntro(
            DrWilyIntro::EndSequence,
        ) => {
            gameworld.entities.retain(|entity| {
                !entity.as_any().is::<BlackFade>()
            });
            gameworld
                .get_entities_by_type::<GameTimer>()
                .first_mut()
                .unwrap()
                .is_active = true;
            // clear the ready entity which signales we are done with the intro sequence
            gameworld.entities.retain(|e| {
                !e.as_any().is::<GPReady>()
            });
        }

        _ => {}
    }
}
async fn run_stage_intro(
    gameworld: &mut GameWorld,
) {
    if let Some(timer) = gameworld
        .get_entities_by_type_and_property(
            |timer: &entities::Timer| {
                timer.name.clone()
            },
            "intro_sequence".to_string(),
        )
        .first_mut()
    {
        if !timer.is_active {
            timer.elapsed_time = 0.;
            timer.is_active = true;
            return;
        }
    }
    // step 1: flash ready until 3 seconds have passed
    let timer_elapsed = gameworld
        .get_entities_by_type_and_property(
            |timer: &entities::Timer| {
                timer.name.clone()
            },
            "intro_sequence".to_string(),
        )
        .first()
        .unwrap()
        .elapsed_time;
    if timer_elapsed <= 3.0 {
        return;
    }
    if let Some(ready_gfx) = gameworld
        .get_entities_by_type::<entities::GPReady>()
        .first_mut()
    {
        if ready_gfx.is_visible && timer_elapsed >= 3.0 {
            ready_gfx.is_visible = false;
        }
    }
    // step 2: warp in megaman
    let (
        megaman_action,
        megaman_position_y,
        megaman_frame_idx,
    ) = gameworld
        .get_entities_by_type::<megaman::MegaMan>(
        )
        .first()
        .map(|megaman| {
            (
                megaman.action,
                megaman.position.y,
                megaman.action_frame_idx,
            )
        })
        .unwrap();
    match megaman_action {
        megaman::Action::Nothing => {
            gameworld
                .get_entities_by_type::<megaman::MegaMan>()
                .first_mut()
                .unwrap()
                .start_action(megaman::Action::WarpingIn);
            gameworld
                .sfx_atlas
                .play(sfx::SFXName::WarpIn)
                .await;
            return;
        }
        megaman::Action::WarpingIn => {
            let y_warp_threshold = {
                match gameworld
                    .get_entities_by_type_and_property(|npc: &NPC| npc.npc_state, NPCState::Gameplay)
                    .first()
                    .unwrap()
                    .name {
                NPCName::RustMan => 25.,
                _ => 18.

                }
            };
            if megaman_position_y
                >= y_warp_threshold
                && megaman_frame_idx == 3
            {
                gameworld
                    .get_entities_by_type::<megaman::MegaMan>()
                    .first_mut()
                    .unwrap()
                    .start_action(megaman::Action::Standing);
                // step 3: robot intro - start trying to intimidate mega man
                gameworld
                    .get_entities_by_type_and_property::<npc::NPC, NPCState>(
                        |robot: &npc::NPC| robot.npc_state,
                        NPCState::Gameplay,
                    )
                    .first_mut()
                    .unwrap()
                    .start_action(NPCAction::Intro);
            }
        }
        megaman::Action::Standing => {}
        _ => return,
    };
    let (robot_action_gfx, robot_action_frame_idx) = gameworld
        .get_entities_by_type_and_property::<npc::NPC, NPCState>(
            |robot: &npc::NPC| robot.npc_state,
            NPCState::Gameplay,
        )
        .first()
        .map(|robot| (robot.npc_action_gfx, robot.npc_action_frame_idx))
        .unwrap();
    // step 4: if we are done intimidating (Reached end of animation sequence), then fill life bar
    let robot_action_texture_frame_length =
        gameworld
            .loaded_textures
            .get(&robot_action_gfx)
            .unwrap_or_else({
                || {
                    panic!(
                        "error loading texture: \
                         {:?}",
                        robot_action_gfx
                    )
                }
            })
            .frames
            .len() as u8;
    let robot_action = gameworld
        .get_entities_by_type_and_property::<npc::NPC, NPCState>(
            |robot: &npc::NPC| robot.npc_state,
            NPCState::Gameplay,
        )
        .first()
        .unwrap()
        .npc_action;
    // if we've reached the end of the animation sequence, then set the robot behavior to standing
    if (robot_action_texture_frame_length
        == robot_action_frame_idx + 1
        && robot_action == NPCAction::Intro)
        || robot_action_texture_frame_length == 1
    {
        gameworld
            .get_entities_by_type_and_property::<npc::NPC, NPCState>(
                |robot: &npc::NPC| robot.npc_state,
                NPCState::Gameplay,
            )
            .first_mut()
            .unwrap()
            .start_action(NPCAction::Standing);
    }
    if robot_action != NPCAction::Standing {
        return;
    }
    // TODO: need to increase accuracy of parity with MM2PA robot intros eg:
    //  heatman last frame while life is filling is different, flashman timing is off, etc.
    gameworld
        .get_entities_by_type_and_property::<npc::NPC, NPCState>(
            |robot: &npc::NPC| robot.npc_state,
            NPCState::Gameplay,
        )
        .first_mut()
        .unwrap()
        .start_action(NPCAction::Standing);
    // make life bar visible
    let (
        life_meter_box_is_visible,
        life_meter_box_qty,
        life_meter_box_frame_elapsed,
    ) = gameworld
        .get_entities_by_type::<LifeMeterBox>()
        .first_mut()
        .map(|life_meter_box| {
            (
                life_meter_box.is_visible,
                life_meter_box.qty,
                life_meter_box.frame_elapsed,
            )
        })
        .unwrap();
    if !life_meter_box_is_visible {
        gameworld
            .get_entities_by_type::<LifeMeterBox>(
            )
            .first_mut()
            .unwrap()
            .is_visible = true;
        gameworld.sfx_atlas.play_looped(
            sfx::SFXName::LifeMeterFill,
        );
    }
    // fill life bar and add life to the robot's hp
    if life_meter_box_frame_elapsed >= 0.09
        && life_meter_box_qty < 16
    {
        // the robot is instantiated with 16 life so we only need to add to the lifebar here
        gameworld
            .get_entities_by_type::<LifeMeterBox>(
            )
            .first_mut()
            .unwrap()
            .frame_elapsed = 0.;
        gameworld
            .get_entities_by_type::<LifeMeterBox>(
            )
            .first_mut()
            .unwrap()
            .qty += 1;
    }
    if life_meter_box_qty == 16 {
        gameworld
            .sfx_atlas
            .stop(sfx::SFXName::LifeMeterFill);
        gameworld
            .get_entities_by_type::<GameTimer>()
            .first_mut()
            .unwrap()
            .is_active = true;
        // clear the ready entity which signales we are done with the intro sequence
        gameworld.entities.retain(|e| {
            !e.as_any().is::<GPReady>()
        });

        // have mega man start blinking
        // TODO: if I want to do random interval blinks, then I need to finish implementing the following:
        // gameworld.get_entities_by_type::<MegaMan>().first_mut().unwrap().start_action(megaman::Action::Standing);
    }
}
async fn initialize(gameworld: &mut GameWorld) {
    // CLEAR RESOURCES LIST
    // gameworld.loaded_textures.clear();
    //gameworld.sfx_atlas.clear();
    // MUST LOAD GFX
    // we use the cursor position from stage select to determine the robot we are facing off against
    let cursor_idx = {
        let cursors = gameworld.get_entities_by_type::<entities::StageSelectCursor>();
        cursors.first().unwrap().index
    };
    // CLEAR ENTITIES LIST
    // retain the robots and game timer but clear the remaining entities so that when we switch game states we dont lose the data
    gameworld.entities.retain(|entity| {
        entity.as_any().is::<NPC>()
            || entity.as_any().is::<GameTimer>()
    });
    // get the robot name to initialize the correct stage
    let robot_name = match cursor_idx {
        1 => npc::NPCName::BubbleMan,
        2 => npc::NPCName::AirMan,
        3 => npc::NPCName::QuickMan,
        4 => npc::NPCName::HeatMan,
        5 => npc::NPCName::DrWily,
        6 => npc::NPCName::WoodMan,
        7 => npc::NPCName::MetalMan,
        8 => npc::NPCName::FlashMan,
        9 => npc::NPCName::CrashMan,
        10 | _ => npc::NPCName::RustMan,
    };
    // set which robot is active - all others inactive
    gameworld
        .get_entities_by_type::<npc::NPC>()
        .iter_mut()
        .for_each(|robot| {
            // set which one is active for this stage
            if robot.name == robot_name {
                robot.npc_state =
                    NPCState::Gameplay;
            } else {
                robot.npc_state =
                    NPCState::Inactive;
            }
        });
    // load assets for each of the robots
    for i in 0..10 {
        gameworld
            .get_entities_by_type::<npc::NPC>()
            .get(i)
            .unwrap()
            .load_all_assets(gameworld)
            .await;
    }
    // z-index for render order:
    // 0. starfield
    // 1. blackfade
    // 2. wily avatar
    // 3. spaceship
    // 4. background
    // 5. ui elements
    // load background
    let mut background_entity =
        entities::GPBackground::new(robot_name)
            .await;
    background_entity.is_visible = true;
    // gameworld.entities.push(Box::new(background_entity));
    gameworld
        .register_entity_first(background_entity)
        .await;
    if gameworld
        .get_entities_by_type_and_property(
            |npc: &NPC| npc.npc_state,
            NPCState::Gameplay,
        )
        .first()
        .unwrap()
        .name
        == NPCName::RustMan
    {
        gameworld
            .register_entity_first(
                entities::Credits::new().await,
            )
            .await;
        gameworld
            .register_entity_first(
                entities::RustCode::new().await,
            )
            .await;
    }
    let spaceship = SpaceShip::new();
    // gameworld.entities.push(Box::new(spaceship));
    gameworld
        .entities
        .insert(0, Box::new(spaceship));
    load_entity_animations!(
        gameworld, &spaceship
    );
    let wily_prop = WilyProp::new();
    gameworld
        .entities
        .insert(0, Box::new(wily_prop));
    load_entity_animations!(
        gameworld, &wily_prop
    );
    // load black fade to cover starfield but behind the background
    let blackfade = BlackFade {
        position: vec2(0., 0.),
        size: DSCREENSIZE,
        color: Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 1.0,
        },
        is_visible: true,
    };
    gameworld
        .entities
        .insert(0, Box::new(blackfade));
    // load lifemeterbox
    let life_meter_box =
        entities::LifeMeterBox::new(robot_name)
            .await;
    // load weapon box select
    let weapon_box_select =
        entities::WeaponBoxSelect::new().await;
    // load READY gameplay
    let mut ready_entity =
        entities::GPReady::new().await;
    ready_entity.is_visible = true;
    // load HitFlash
    let hitflash = HitFlash::new();
    // These entities need their animations loaded, but not be added to the entity list
    load_entity_animations!(
        gameworld,
        &background_entity,
        &life_meter_box,
        &weapon_box_select,
        &ready_entity
    );
    register_entities!(gameworld, hitflash);
    load_animations!(
        gameworld,
        gfx::Name::STARlarge,
        gfx::Name::STARmedium,
        gfx::Name::STARsmall
    );
    // used on tetris grid point when clearing lines
    load_animations!(
        gameworld,
        gfx::Name::GPpshooterBlockFlashing,
        gfx::Name::GPpshooterBlockBright
    );
    // when robot explodes do explodey things
    load_animations!(
        gameworld,
        gfx::Name::RobotExplosionOrb
    );
    // load gameplay MegaMan
    let mut megaman =
        megaman::MegaMan::new().await;
    megaman.action = megaman::Action::Nothing;
    megaman.load_all_assets(gameworld).await;
    megaman.position = vec2(16., -31.);
    // create an intro sequence timer so we know if the READY sequence has flashed long enough
    let timer =
        entities::Timer::new("intro_sequence")
            .await;
    // create the game timer for best times if not carrying one over
    if gameworld
        .get_entities_by_type::<GameTimer>()
        .is_empty()
    {
        let game_timer =
            entities::GameTimer::new().await;
        register_entities!(gameworld, game_timer);
    } else {
        gameworld
            .get_entities_by_type::<GameTimer>()
            .first_mut()
            .unwrap()
            .is_visible = true;
    }
    // create the tetris grid and pieces
    let tetris_grid =
        resources::entities::TetrisGrid::new();
    // LOAD DATA files
    //  none.
    // load conditional misc music
    let defeat_music = music::MusicTrack::load(
        music::TrackName::Defeat,
    )
    .await;
    let victory_music = music::MusicTrack::load(
        music::TrackName::RobotVictory,
    )
    .await;
    // TODO: only load this for wily stage
    let wily_victory_music =
        music::MusicTrack::load(
            music::TrackName::WilyVictory,
        )
        .await;
    // load condition misc gfx
    // TODO: check if we can remove this explosion orb / move it to the megaman load all assets function
    load_animations!(
        gameworld,
        gfx::Name::MegaManExplosionOrb
    );
    register_entities!(
        gameworld,
        victory_music,
        defeat_music,
        wily_victory_music
    );
    // TODO: we could condense this by only declaring variables inside match statement and moving rest outside of match statement
    match robot_name {
        NPCName::RustMan => {
            let mut looped =
                music::MusicTrack::load(
                    music::TrackName::RustMan,
                )
                .await;
            looped.params.looped = true;
            looped.play();
            register_entities!(gameworld, looped);
        }

        NPCName::BubbleMan => {
            let mut intro = music::MusicTrack::load(music::TrackName::BubbleManIntro).await;
            let mut looped = music::MusicTrack::load(music::TrackName::BubbleManLoop).await;
            looped.params.looped = true;
            intro.play();
            register_entities!(
                gameworld, intro, looped
            );
        }

        NPCName::AirMan => {
            let mut looped =
                music::MusicTrack::load(
                    music::TrackName::AirMan,
                )
                .await;
            looped.params.looped = true;
            looped.play();
            register_entities!(gameworld, looped);
        }

        NPCName::QuickMan => {
            let mut looped =
                music::MusicTrack::load(
                    music::TrackName::QuickMan,
                )
                .await;
            looped.params.looped = true;
            looped.play();
            register_entities!(gameworld, looped);
        }

        NPCName::HeatMan => {
            let mut looped =
                music::MusicTrack::load(
                    music::TrackName::HeatMan,
                )
                .await;
            looped.params.looped = true;
            looped.play();
            register_entities!(gameworld, looped);
        }

        NPCName::WoodMan => {
            let mut intro = music::MusicTrack::load(music::TrackName::WoodManIntro).await;
            let mut looped =
                music::MusicTrack::load(
                    music::TrackName::WoodManLoop,
                )
                .await;
            looped.params.looped = true;
            intro.play();
            register_entities!(
                gameworld, intro, looped
            );
        }

        NPCName::MetalMan => {
            let mut looped =
                music::MusicTrack::load(
                    music::TrackName::MetalMan,
                )
                .await;
            looped.params.looped = true;
            looped.play();
            register_entities!(gameworld, looped);
        }

        NPCName::FlashMan => {
            let mut intro = music::MusicTrack::load(music::TrackName::FlashManIntro).await;
            let mut looped = music::MusicTrack::load(music::TrackName::FlashManLoop).await;
            looped.params.looped = true;
            intro.play();
            register_entities!(
                gameworld, intro, looped
            );
        }

        NPCName::CrashMan => {
            let mut intro = music::MusicTrack::load(music::TrackName::CrashManIntro).await;
            let mut looped = music::MusicTrack::load(music::TrackName::CrashManLoop).await;
            looped.params.looped = true;
            intro.play();
            register_entities!(
                gameworld, intro, looped
            );
        }

        NPCName::DrWily => {
            let mut intro =
                music::MusicTrack::load(
                    music::TrackName::Wily1_2,
                )
                .await;
            intro.params.looped = true;
            intro.play();
            let mut looped = music::MusicTrack::load(music::TrackName::QGMWilyAltered).await;
            looped.params.looped = true;
            register_entities!(
                gameworld, looped, intro
            );
        }
    }
    // MUST LOAD SFX
    gameworld
        .sfx_atlas
        .add_sfx(sfx::SFXName::WarpIn)
        .await;
    gameworld
        .sfx_atlas
        .add_sfx(sfx::SFXName::LifeMeterFill)
        .await;
    gameworld
        .sfx_atlas
        .add_sfx(sfx::SFXName::RobotHit)
        .await;
    gameworld
        .sfx_atlas
        .add_sfx(sfx::SFXName::RobotDeath)
        .await;
    gameworld
        .sfx_atlas
        .add_sfx(sfx::SFXName::WarpIn)
        .await;
    gameworld
        .sfx_atlas
        .add_sfx(sfx::SFXName::WarpOut)
        .await;
    gameworld
        .sfx_atlas
        .add_sfx(sfx::SFXName::SpaceShip)
        .await;
    // ROBOT SFX
    gameworld
        .sfx_atlas
        .add_sfx(sfx::SFXName::PShot)
        .await;
    // SET CAMERA
    // TODO: test Camera on RustMan Stage
    // gameworld.camera.target = Vec2 { x: DSCREENSIZE.x / 2., y: DSCREENSIZE.y / 2.};
    // set_camera(&gameworld.camera);
    // CREATE ENTITIES
    //  the order we add the graphics affects the render order
    register_entities!(
        gameworld,
        ready_entity,
        life_meter_box,
        megaman,
        timer,
        tetris_grid
    );
    // because the npc weapon select graphics carry over from previous stages we need to insert
    //  the weapon_box_select _behind_ them but in front of the background.
    gameworld
        .entities
        .insert(1, Box::new(weapon_box_select));
    if robot_name == NPCName::RustMan {
        gameworld
            .get_entities_by_type_and_property(
                |npc: &NPC| npc.npc_state,
                NPCState::Gameplay,
            )
            .first_mut()
            .unwrap()
            .npc_avatar_pos += 8.;
    }
    // SET INIT = TRUE
    gameworld.gamestate.is_initialized = true;
}
/// VICTORY - Run Sequence when robot is defeated
/// 0. stop music, stop clock
/// 1. robot explodes, sfx plays
/// 2. waiting for final [ExplosionOrb] to go off screen
/// 3. play victory music
/// 4. warp MM out, play warp sfx
/// 5. Fade/Flash in the colored robot weapon
/// line 3284 in mm2pa.py
async fn run_robot_defeated(
    gameworld: &mut GameWorld,
) {
    // TODO: change this to use the SequenceManager instead of Magic Numbers
    // lets create a timer object that lets the main game play game state loop know not to do stuff
    if gameworld
        .get_entities_by_type_and_property(
            |timer: &entities::Timer| {
                timer.name.clone()
            },
            "robot_defeated_sequence_timer"
                .to_string(),
        )
        .is_empty()
    {
        let mut robot_defeated_sequence_timer =
            entities::Timer::new(
                "robot_defeated_sequence_timer",
            )
            .await;
        robot_defeated_sequence_timer
            .elapsed_time = 1.;
        register_entities!(
            gameworld,
            robot_defeated_sequence_timer
        );
    }
    // else we get the current value for the sequence
    let sequence_step = gameworld
        .get_entities_by_type_and_property(
            |timer: &entities::Timer| {
                timer.name.clone()
            },
            "robot_defeated_sequence_timer"
                .to_string(),
        )
        .first()
        .unwrap()
        .elapsed_time
        as usize;
    // ..then we check the conditions for advancing the sequence steps in reverse order to make sure wenever regress:
    // before we can go to step 3, we have to check if all the explosion orbs are off the screen
    if sequence_step < 3 {
        let active_explosion_orbs = gameworld
            .get_entities_by_type::<ExplosionOrb>(
        );
        let orbs_clear_of_screen =
            active_explosion_orbs.iter().all(
                |orb| {
                    (orb.position.x
                        > DSCREENSIZE.x
                        || orb.position.x < 0.)
                        || (orb.position.y
                            > DSCREENSIZE.y
                            || orb.position.y
                                < 0.)
                },
            );
        if orbs_clear_of_screen {
            gameworld
                .get_entities_by_type_and_property(
                    |timer: &entities::Timer| timer.name.clone(),
                    "robot_defeated_sequence_timer".to_string(),
                )
                .first_mut()
                .unwrap()
                .elapsed_time = 3.;
        }
    }
    // Run Sequence
    if sequence_step == 1 {
        // 0. stop music
        gameworld
            .get_entities_by_type::<MusicTrack>()
            .iter_mut()
            .for_each(|track| {
                track.stop();
            });
        if let Some(game_timer) = gameworld
            .get_entities_by_type_and_property(
                |timer: &GameTimer| {
                    timer.is_active
                },
                true,
            )
            .first_mut()
        {
            game_timer.is_active = false;
        }
        // 1. robot explodes, sfx plays
        if gameworld
            .get_entities_by_type::<ExplosionOrb>(
            )
            .is_empty()
        {
            let gfx = gameworld
                .get_entities_by_type_and_property(|npc: &NPC| npc.npc_state, NPCState::Gameplay)
                .first_mut()
                .unwrap()
                .npc_action_gfx;
            // to position the explosion orbs at the center of the robot we first get the robot's position
            //  which is actually the uppper left corner where its sprite gets started drawing
            let mut origin = gameworld
                .get_entities_by_type_and_property(|npc: &NPC| npc.npc_state, NPCState::Gameplay)
                .first_mut()
                .unwrap()
                .npc_avatar_pos;
            // then we add half the sprite's width and subtract half its height to get the center of the robot
            origin.y -= gameworld
                .loaded_textures
                .get(&gfx)
                .unwrap()
                .frames
                .first()
                .unwrap()
                .size()
                .y
                / 2.;
            origin.x += gameworld
                .loaded_textures
                .get(&gfx)
                .unwrap()
                .frames
                .first()
                .unwrap()
                .size()
                .x
                / 2.;
            // however, since sprites are drawn from the top left and down to the right, we have to adjust for
            //  the size of the explosion orb's height and width, so we subtract half of its dimensions
            if let Some(orb_texture) =
                gameworld.loaded_textures.get(
                    &gfx::Name::RobotExplosionOrb,
                )
            {
                if let Some(orb_texture_frame) =
                    orb_texture.frames.first()
                {
                    origin -= orb_texture_frame
                        .size()
                        / 2.;
                }
            }
            // now we can finally spawn the orbs with the proper position
            let variation = {
                if gameworld
                    .get_entities_by_type_and_property(
                        |npc: &NPC| npc.npc_state,
                        NPCState::Gameplay,
                    )
                    .first()
                    .unwrap()
                    .name
                    == NPCName::DrWily
                {
                    Some(NPCName::DrWily)
                } else {
                    None
                }
            };
            let orbs = explosion_orb::spawn_explosion_orbs(variation, origin, 8);
            for orb in orbs {
                register_entities!(
                    gameworld, orb
                );
            }
            // play explosion sound
            gameworld
                .sfx_atlas
                .play(sfx::SFXName::RobotDeath)
                .await;
            gameworld
                .get_entities_by_type_and_property(
                    |timer: &entities::Timer| timer.name.clone(),
                    "robot_defeated_sequence_timer".to_string(),
                )
                .first_mut()
                .unwrap()
                .elapsed_time = 2.;
        }
    }
    // play victory music
    if sequence_step == 3 {
        // if wily do wily stuff
        if gameworld
            .get_entities_by_type_and_property(
                |npc: &NPC| npc.npc_state,
                NPCState::Gameplay,
            )
            .first()
            .unwrap()
            .name
            == NPCName::DrWily
        {
            if let Some(victory_music_track) = gameworld
                .get_entities_by_type_and_property(
                    |track: &MusicTrack| track.name.clone(),
                    TrackName::WilyVictory,
                )
                .first_mut()
            {
                if victory_music_track.elapsed == 0. {
                    victory_music_track.play();
                }

                if victory_music_track.elapsed >= 10.0 {
                    victory_music_track.stop();
                    gameworld
                        .get_entities_by_type_and_property(
                            |timer: &entities::Timer| timer.name.clone(),
                            "robot_defeated_sequence_timer".to_string(),
                        )
                        .first_mut()
                        .unwrap()
                        .elapsed_time = 4.
                }
            }
        }
        // else do normal robot stuff
        else {
            if let Some(victory_music_track) = gameworld
                .get_entities_by_type_and_property(
                    |track: &MusicTrack| track.name.clone(),
                    TrackName::RobotVictory,
                )
                .first_mut()
            {
                if victory_music_track.elapsed == 0. {
                    victory_music_track.play();
                }

                if victory_music_track.elapsed >= 5.0 {
                    victory_music_track.stop();
                    gameworld
                        .get_entities_by_type_and_property(
                            |timer: &entities::Timer| timer.name.clone(),
                            "robot_defeated_sequence_timer".to_string(),
                        )
                        .first_mut()
                        .unwrap()
                        .elapsed_time = 4.
                }
            }
        }
    }
    // warp mm out
    if sequence_step == 4 {
        if let Some(megaman) = gameworld
            .get_entities_by_type::<MegaMan>()
            .first_mut()
        {
            megaman.start_action(
                megaman::Action::WarpingOut,
            );
            gameworld
                .sfx_atlas
                .play(sfx::SFXName::WarpOut)
                .await;
            // gameworld.get_entities_by_type::<NPC>().iter_mut().for_each(|npc| {
            //     npc.start_action(NPCAction::Fading { alpha: 1. });
            // });
            gameworld
                .get_entities_by_type_and_property(
                    |timer: &entities::Timer| timer.name.clone(),
                    "robot_defeated_sequence_timer".to_string(),
                )
                .first_mut()
                .unwrap()
                .elapsed_time = 5.
        }
    }
    // flash weapon face to color
    if sequence_step == 5 {
        if gameworld
            .get_entities_by_type::<MegaMan>()
            .first_mut()
            .unwrap()
            .position
            .y
            <= -60.
        {
            // if dr. wily, fade stage and ui instead
            if gameworld
                .get_entities_by_type_and_property(|npc: &NPC| npc.npc_state, NPCState::Gameplay)
                .first()
                .unwrap()
                .name
                == NPCName::DrWily
            {
                let fade_qty = get_frame_time() / 3.;

                // fade stage
                gameworld
                    .get_entities_by_type::<entities::GPBackground>()
                    .first_mut()
                    .unwrap()
                    .alpha -= fade_qty;

                // begin fade NPC UI elements for Dr.wily
                gameworld
                    .get_entities_by_type::<NPC>()
                    .iter_mut()
                    .for_each(|npc| {
                        npc.gp_alpha -= fade_qty;
                    });

                // fade the weaponselect box
                gameworld
                    .get_entities_by_type::<WeaponBoxSelect>()
                    .first_mut()
                    .unwrap()
                    .alpha -= fade_qty;

                // fade the mega man face
                gameworld
                    .get_entities_by_type::<MegaMan>()
                    .first_mut()
                    .unwrap()
                    .alpha -= fade_qty;

                // fade the game timer
                gameworld
                    .get_entities_by_type::<GameTimer>()
                    .first_mut()
                    .unwrap()
                    .is_visible = false;
                gameworld
                    .get_entities_by_type::<LifeMeterBox>()
                    .first_mut()
                    .unwrap()
                    .is_visible = false;

                // fade the tetris grid points
                if let Some(tetris_grid) =
                    gameworld.get_entities_by_type::<TetrisGrid>().first_mut()
                {
                    tetris_grid.matrix.iter_mut().for_each(|gp| {
                        gp.alpha -= fade_qty;
                    });
                }

                if gameworld
                    .get_entities_by_type::<entities::GPBackground>()
                    .first_mut()
                    .unwrap()
                    .alpha
                    <= 0.
                {
                    gameworld
                        .get_entities_by_type_and_property(
                            |timer: &entities::Timer| timer.name.clone(),
                            "robot_defeated_sequence_timer".to_string(),
                        )
                        .first_mut()
                        .unwrap()
                        .elapsed_time = 6.
                }
            } else {
                gameworld
                    .get_entities_by_type::<NPC>()
                    .iter_mut()
                    .find(|npc| npc.npc_state == NPCState::Gameplay)
                    .unwrap()
                    .start_action(NPCAction::WeaponInstalling);
                gameworld
                    .get_entities_by_type_and_property(
                        |timer: &entities::Timer| timer.name.clone(),
                        "robot_defeated_sequence_timer".to_string(),
                    )
                    .first_mut()
                    .unwrap()
                    .elapsed_time = 6.
            }
        }
    }

    // end game state and return to stage select
    // if sequence_step == 6 {
    //     if gameworld.get_entities_by_type::<NPC>().iter_mut().find(|npc| npc.npc_state == NPCState::Gameplay).unwrap().npc_action_frame_idx == 41 {

    //     }
    // }
}
/// FAIL - Run Failure Sequence
async fn fail_sequence_start(
    gameworld: &mut GameWorld,
) -> SequenceName {
    // stop game timer
    if let Some(game_timer) = gameworld
        .get_entities_by_type_and_property(
            |timer: &GameTimer| timer.is_active,
            true,
        )
        .iter_mut()
        .next()
    {
        game_timer.is_active = false;
    }
    // stop all music playing
    gameworld
        .get_entities_by_type::<MusicTrack>()
        .iter_mut()
        .for_each(|track| {
            track.stop();
        });
    // start mega man exploding behavior and play sfx
    if let Some(megaman) = gameworld
        .get_entities_by_type::<MegaMan>()
        .first_mut()
    {
        if megaman.action
            != megaman::Action::Exploding
        {
            megaman.start_action(
                megaman::Action::Exploding,
            );
        }
    };
    // show explosion orbs
    if gameworld
        .get_entities_by_type::<ExplosionOrb>()
        .is_empty()
    {
        let gfx = gameworld
            .get_entities_by_type::<MegaMan>()
            .first()
            .unwrap()
            .gfx_name;
        // to position the explosion orbs at the center of the robot we first get the robot's position
        //  which is actually the uppper left corner where its sprite gets started drawing
        let mut origin = gameworld
            .get_entities_by_type::<MegaMan>()
            .first()
            .unwrap()
            .position;
        // then we add half the sprite's width and subtract half its height to get the center
        origin.y += gameworld
            .loaded_textures
            .get(&gfx)
            .unwrap()
            .frames
            .first()
            .unwrap()
            .size()
            .y
            / 2.;
        origin.x += gameworld
            .loaded_textures
            .get(&gfx)
            .unwrap()
            .frames
            .first()
            .unwrap()
            .size()
            .x
            / 2.;
        // however, since sprites are drawn from the top left and down to the right, we have to adjust for
        //  the size of the explosion orb's height and width, so we subtract half of its dimensions
        if let Some(orb_texture) = gameworld
            .loaded_textures
            .get(&gfx::Name::RobotExplosionOrb)
        {
            if let Some(orb_texture_frame) =
                orb_texture.frames.first()
            {
                origin -=
                    orb_texture_frame.size() / 2.;
            }
        }
        let active_weapon = gameworld
            .get_entities_by_type::<MegaMan>()
            .first()
            .unwrap()
            .active_weapon;
        // now we can finally spawn the orbs with the proper position
        let orbs =
            explosion_orb::spawn_explosion_orbs(
                active_weapon,
                origin,
                8,
            );
        for mut orb in orbs {
            gameworld.register_entity(orb).await;
        }
        // play explosion sound
        gameworld
            .sfx_atlas
            .play(sfx::SFXName::RobotDeath)
            .await;
    }
    return SequenceName::GamePlayDefeat(
        Defeat::WaitForOrbsToClear,
    );
}
async fn fail_sequence_wait_for_orbs(
    gameworld: &mut GameWorld,
) -> SequenceName {
    let orb_size = {
        let active_explosion_orbs = gameworld
            .get_entities_by_type::<ExplosionOrb>(
        );
        let gfx_name = active_explosion_orbs
            .first()
            .unwrap()
            .gfx_name
            .clone();
        gameworld
            .loaded_textures
            .get(&gfx_name)
            .unwrap()
            .frames
            .first()
            .unwrap()
            .size()
            .clone()
    };
    let active_explosion_orbs = gameworld
        .get_entities_by_type::<ExplosionOrb>(
    );
    let orbs_clear_of_screen =
        active_explosion_orbs.iter().all(|orb| {
            orb.position.x > DSCREENSIZE.x
                || orb.position.x
                    < 0. - orb_size.x
                || orb.position.y > DSCREENSIZE.y
                || orb.position.y
                    < 0. - orb_size.y
        });
    if orbs_clear_of_screen {
        return SequenceName::GamePlayDefeat(
            Defeat::PlayMusic,
        );
    } else {
        return SequenceName::GamePlayDefeat(
            Defeat::WaitForOrbsToClear,
        );
    }
}
async fn fail_sequence_play_music(
    gameworld: &mut GameWorld,
) -> SequenceName {
    if let Some(fail_music_track) = gameworld
        .get_entities_by_type_and_property(
            |track: &MusicTrack| {
                track.name.clone()
            },
            TrackName::Defeat,
        )
        .first_mut()
    {
        if fail_music_track.elapsed == 0. {
            fail_music_track.play();
        }
        // Fail music last just 3.0 seconds
        if fail_music_track.elapsed >= 3.0 {
            fail_music_track.stop();
            SequenceName::GamePlayDefeat(
                Defeat::EndSequence,
            )
        } else {
            SequenceName::GamePlayDefeat(
                Defeat::PlayMusic,
            )
        }
    } else {
        SequenceName::GamePlayDefeat(
            Defeat::PlayMusic,
        )
    }
}
async fn run_fail_sequence(
    gameworld: &mut GameWorld,
) -> GameState {
    // get or create the sequencing struct for the failure sequence
    let defeat_sequence_opt: Option<
        &mut Sequencer,
    > = gameworld
        .get_entities_by_type_and_property(
            |sequencer: &Sequencer| {
                matches!(
                    sequencer.sequence,
                    SequenceName::GamePlayDefeat(
                        _
                    )
                )
            },
            true,
        )
        // TODO: TIL that `.into_iter().next()` may be a better way of getting a mutable item out of the
        //  entities list than .first_mut() or .first() - need to consider retroactively cleaning up code that
        .into_iter()
        .next();
    let mut defeat_sequence: Sequencer = {
        if let Some(existing_seq) = defeat_sequence_opt {
            existing_seq
        } else {
            // No existing sequencer found, so create, register, and then retrieve it.
            let new_sequencer = Sequencer {
                sequence: SequenceName::GamePlayDefeat(Defeat::Initialize),
            };
            gameworld.register_entity(new_sequencer).await;

            gameworld
                .get_entities_by_type_and_property(
                    |sequencer: &Sequencer| {
                        matches!(
                            sequencer.sequence,
                            SequenceName::GamePlayDefeat(Defeat::Initialize)
                        )
                    },
                    true,
                )
                .into_iter()
                .next()
                .expect("Failed to retrieve newly added defeat sequencer after registration")
        }
    }
    .clone();
    // do different stuff depending on where we are in the sequence
    defeat_sequence.sequence =
        match defeat_sequence.sequence {
            SequenceName::GamePlayDefeat(
                Defeat::Initialize,
            )
            | SequenceName::GamePlayDefeat(
                Defeat::StartSequence,
            ) => {
                fail_sequence_start(gameworld)
                    .await
            }
            SequenceName::GamePlayDefeat(
                Defeat::WaitForOrbsToClear,
            ) => {
                fail_sequence_wait_for_orbs(
                    gameworld,
                )
                .await
            }
            SequenceName::GamePlayDefeat(
                Defeat::PlayMusic,
            ) => {
                fail_sequence_play_music(
                    gameworld,
                )
                .await
            }
            SequenceName::GamePlayDefeat(
                Defeat::EndSequence,
            )
            | _ => {
                return GameState {
                    state: State::StageSelect,
                    is_initialized: false,
                };
            } // _ => {}
        };
    // update the sequence state
    if let Some(sequence) = gameworld
        .get_entities_by_type_and_property(
            |sequencer: &Sequencer| {
                matches!(
                    sequencer.sequence,
                    SequenceName::GamePlayDefeat(
                        _
                    )
                )
            },
            true,
        )
        .first_mut()
    {
        sequence.sequence =
            defeat_sequence.sequence;
    }
    return GameState {
        state: State::Gameplay,
        is_initialized: true,
    };
}
// HELPER FUNCTIONS
pub async fn switch_weapon(
    gameworld: &mut GameWorld,
    weapon: Option<npc::NPCName>,
) {
    let switch_weapon_sequence = Sequencer {
        sequence:
            SequenceName::GamePlaySwitchWeapon(
                SwitchWeapon::Initialize,
            ),
    };
    register_entities!(
        gameworld,
        switch_weapon_sequence
    );
    gameworld
        .sfx_atlas
        .play(sfx::SFXName::WarpIn)
        .await;
    gameworld
        .get_entities_by_type::<MegaMan>()
        .into_iter()
        .next()
        .unwrap()
        .switch_active_weapon(weapon);
    gameworld
        .get_entities_by_type::<WeaponBoxSelect>()
        .into_iter()
        .next()
        .unwrap()
        .select(weapon);
    gameworld
        .get_entities_by_type::<TetrisGrid>()
        .into_iter()
        .next()
        .unwrap()
        .matrix
        .iter_mut()
        .for_each(|gp| {
            gp.switch_grid_point_gfx(weapon)
        });
    if gameworld
        .get_entities_by_type::<TetrisPiece>()
        .iter_mut()
        .next()
        .is_some()
    {
        gameworld
            .get_entities_by_type::<TetrisPiece>()
            .iter_mut()
            .next()
            .unwrap()
            .piece_type_shape
            .iter_mut()
            .for_each(|gp| {
                gp.switch_grid_point_gfx(weapon)
            });
    }
}
/// find lines to clear on the tetris grid, make them flash, find flashing lines, make them empty, move supported blocks down, do damage
async fn clear_lines(gameworld: &mut GameWorld) {
    let mut new_rows_cleared: u8 = 0;
    // find lines to clear and make them flash
    if let Some(tetris_grid) = gameworld
        .get_entities_by_type::<TetrisGrid>()
        .first_mut()
    {
        tetris_grid
            .matrix
            .chunks_mut(10)
            .filter(|row| {
                row.iter()
                    .all(|gp| gp.point_state == TetrisGridPointState::Active)
            })
            .for_each(|row| {
                new_rows_cleared += 1;
                row.iter_mut().rev().for_each(|gp| {
                    gp.switch_grid_point_state(TetrisGridPointState::Flashing);
                })
            });
        // reset timer
        if new_rows_cleared > 0 {
            tetris_grid.gfx_time_elapsed = 0.;
        }
    }
    // find the bottom most flashing line, make the grid points empty,
    // .. load the index of supported grid points into `falling_grid_points` to be moved down
    let mut falling_grid_points: Vec<usize> =
        Vec::new();
    let tetris_grid_time_elapsed = gameworld
        .get_entities_by_type::<TetrisGrid>()
        .first()
        .unwrap()
        .gfx_time_elapsed
        .clone();
    if tetris_grid_time_elapsed >= 0.25 {
        if let Some(tetris_grid) = gameworld
            .get_entities_by_type::<TetrisGrid>()
            .first_mut()
        {
            tetris_grid
                .matrix
                .chunks_mut(10)
                .rev()
                .find(|row| {
                    row.iter()
                        .all(|gp| gp.point_state == TetrisGridPointState::Flashing)
                })
                .iter_mut()
                .for_each(|row| {
                    row.iter_mut().for_each(|gp| {
                        if gp.point_state == TetrisGridPointState::Flashing {
                            // gp.point_state = TetrisGridPointState::Empty;
                            // gp.point_active_gfx = gfx::Name::GPpshooterBlock;
                            gp.switch_grid_point_state(TetrisGridPointState::Empty);

                            falling_grid_points.push(gp.point_idx as usize);
                        }
                    })
                });
        }
        if !falling_grid_points.is_empty() {
            if let Some(tetris_grid) = gameworld.get_entities_by_type::<TetrisGrid>().first_mut() {
                let idx_max = falling_grid_points.iter().max().unwrap().clone();
                for idx in (10..idx_max + 1).rev() {
                    tetris_grid.matrix[idx].point_state = tetris_grid.matrix[idx - 10].point_state;
                    tetris_grid.matrix[idx].point_active_gfx =
                        tetris_grid.matrix[idx - 10].point_active_gfx;
                }
                tetris_grid.gfx_time_elapsed = 0.;
            }
        } else {
            gameworld
                .get_entities_by_type::<HitFlash>(
                )
                .first_mut()
                .unwrap()
                .is_visible = false;
            gameworld
                .get_entities_by_type::<HitFlash>(
                )
                .first_mut()
                .unwrap()
                .is_active = false;
        }
    }
    // do some damage
    if new_rows_cleared > 0 {
        let active_weapon = gameworld
            .get_entities_by_type::<MegaMan>()
            .first()
            .unwrap()
            .active_weapon
            .clone();
        let current_npc = gameworld
            .get_entities_by_type::<NPC>()
            .iter()
            .find(|npc| {
                npc.npc_state
                    == NPCState::Gameplay
            })
            .unwrap()
            .name
            .clone();
        let calculated_damage = calculate_damage(
            new_rows_cleared,
            active_weapon,
            current_npc,
        );
        // gameworld.get_entities_by_type::<NPC>().iter_mut().find(|npc| npc.npc_state == NPCState::Gameplay).unwrap().take_damage(16);
        gameworld
            .get_entities_by_type::<NPC>()
            .iter_mut()
            .find(|npc| {
                npc.npc_state
                    == NPCState::Gameplay
            })
            .unwrap()
            .take_damage(calculated_damage);
        let new_life_meter_value = gameworld
            .get_entities_by_type_and_property(
                |npc: &NPC| npc.npc_state,
                NPCState::Gameplay,
            )
            .first_mut()
            .unwrap()
            .hitpoints;
        gameworld
            .get_entities_by_type::<LifeMeterBox>(
            )
            .first_mut()
            .unwrap()
            .qty = new_life_meter_value;
        gameworld
            .sfx_atlas
            .play(sfx::SFXName::RobotHit)
            .await;
        if let Some(hitflash) = gameworld
            .get_entities_by_type::<HitFlash>()
            .first_mut()
        {
            if !hitflash.is_active {
                hitflash.is_active = true;
            }
        }
        if gameworld
            .get_entities_by_type_and_property(
                |npc: &NPC| npc.npc_state,
                NPCState::Gameplay,
            )
            .first_mut()
            .unwrap()
            .hitpoints
            == 0
        {
            gameworld
                .get_entities_by_type::<HitFlash>(
                )
                .first_mut()
                .unwrap()
                .is_visible = false;
            gameworld
                .get_entities_by_type::<HitFlash>(
                )
                .first_mut()
                .unwrap()
                .is_active = false;
            gameworld
                .get_entities_by_type::<NPC>()
                .iter_mut()
                .find(|npc| {
                    npc.npc_state
                        == NPCState::Gameplay
                })
                .unwrap()
                .start_action(
                    NPCAction::Exploding,
                );
        }
    }
}
// see: https://www.mmhp.net/GameHints/MM2-Data.html#:~:text=Weapon%3A%20Bubble%20Lead,Bubble%20Man
// see: https://docs.google.com/spreadsheets/d/1OLRnn-jMqGKO3bz4UQZqW8MhOBR-eRSZAvLogVualks/edit?gid=0#gid=0
/// Calculates damage based on currently active weapon versus current robot stage and number of lines cleared
/// takes rows as u8, enum, and enum
/// returns u8 of damage
fn calculate_damage(
    rows: u8, active_weapon: Option<NPCName>,
    current_npc: NPCName,
) -> u8 {
    // DEBUG:
    return 16;
    let multiplier =
        match (active_weapon, current_npc) {
            // P Shooter
            (None, NPCName::DrWily) => 0,

            (
                None,
                NPCName::AirMan
                | NPCName::QuickMan
                | NPCName::HeatMan
                | NPCName::FlashMan,
            ) => 2,
            (None, _) => 1,

            // Bubble Weapon
            (
                Some(NPCName::BubbleMan),
                NPCName::WoodMan
                | NPCName::AirMan
                | NPCName::QuickMan
                | NPCName::MetalMan,
            ) => 0,
            (
                Some(NPCName::BubbleMan),
                NPCName::FlashMan
                | NPCName::DrWily,
            ) => 2,
            (
                Some(NPCName::BubbleMan),
                NPCName::HeatMan,
            ) => 4,

            (
                Some(NPCName::AirMan),
                NPCName::BubbleMan
                | NPCName::MetalMan
                | NPCName::FlashMan
                | NPCName::DrWily,
            ) => 0,
            (
                Some(NPCName::AirMan),
                NPCName::QuickMan
                | NPCName::HeatMan,
            ) => 2,
            (
                Some(NPCName::AirMan),
                NPCName::WoodMan,
            ) => 3,
            (
                Some(NPCName::AirMan),
                NPCName::CrashMan,
            ) => 4,

            (
                Some(NPCName::QuickMan),
                NPCName::WoodMan
                | NPCName::FlashMan
                | NPCName::DrWily,
            ) => 0,
            (
                Some(NPCName::QuickMan),
                NPCName::AirMan
                | NPCName::HeatMan,
            ) => 2,
            (
                Some(NPCName::QuickMan),
                NPCName::MetalMan,
            ) => 3,

            (
                Some(NPCName::HeatMan),
                NPCName::BubbleMan
                | NPCName::DrWily,
            ) => 0,
            (
                Some(NPCName::HeatMan),
                NPCName::AirMan
                | NPCName::QuickMan
                | NPCName::FlashMan,
            ) => 2,
            (
                Some(NPCName::HeatMan),
                NPCName::WoodMan,
            ) => 3,

            (
                Some(NPCName::WoodMan),
                NPCName::BubbleMan
                | NPCName::QuickMan
                | NPCName::HeatMan
                | NPCName::MetalMan
                | NPCName::FlashMan
                | NPCName::CrashMan
                | NPCName::DrWily,
            ) => 0,
            (
                Some(NPCName::WoodMan),
                NPCName::AirMan,
            ) => 4,

            (
                Some(NPCName::MetalMan),
                NPCName::AirMan
                | NPCName::QuickMan
                | NPCName::CrashMan
                | NPCName::DrWily,
            ) => 0,
            (
                Some(NPCName::MetalMan),
                NPCName::WoodMan,
            ) => 2,
            (
                Some(NPCName::MetalMan),
                NPCName::BubbleMan
                | NPCName::FlashMan,
            ) => 3,

            (
                Some(NPCName::FlashMan),
                NPCName::BubbleMan
                | NPCName::AirMan
                | NPCName::HeatMan
                | NPCName::WoodMan
                | NPCName::MetalMan
                | NPCName::FlashMan
                | NPCName::CrashMan
                | NPCName::DrWily,
            ) => 0,
            (
                Some(NPCName::FlashMan),
                NPCName::QuickMan,
            ) => 4,

            (
                Some(NPCName::CrashMan),
                NPCName::AirMan
                | NPCName::HeatMan
                | NPCName::MetalMan
                | NPCName::CrashMan
                | NPCName::DrWily,
            ) => 0,
            (
                Some(NPCName::CrashMan),
                NPCName::BubbleMan
                | NPCName::QuickMan
                | NPCName::WoodMan,
            ) => 2,
            (
                Some(NPCName::CrashMan),
                NPCName::FlashMan,
            ) => 3,

            // TODO: custom calcs for rustman
            (_, _) => 1,
        };
    rows * multiplier
}
/// check if a new tetris piece can be placed in the tetris grid in order to check losing condition
fn is_piece_overlapping_grid_points(
    gameworld: &mut GameWorld,
    piece: &TetrisPiece,
) -> bool {
    let tetris_grid_points = gameworld
        .get_entities_by_type::<TetrisGrid>()
        .first()
        .unwrap()
        .matrix
        .clone();
    // check if any of the tetris grid points are active that the tetris piece would occupy
    !piece.piece_type_shape.iter().any(|gp| {
        tetris_grid_points[gp.point_idx as usize]
            .point_state
            == TetrisGridPointState::Active
    })
}
/// check if any of the active tetris piece's grid points are out of bounds vertically (negative y value)
// TODO: check if a placed piece has any point above the playfield
fn is_piece_above_grid_points(
    piece: &TetrisPiece,
) -> bool {
    // get highest (lowest y value) of active tetris piece. If its negative, then return true
    let lowest_grid_point = piece
        .piece_type_shape
        .iter()
        .map(|gp| {
            entities::idx_to_grid_point(
                gp.point_idx,
            )
            .y
        })
        .min()
        .unwrap()
        .clone();
    lowest_grid_point < 0
}
/// Check if the active tetris piece has collided with another piece or the bottom row
fn process_collisions(
    gameworld: &mut GameWorld,
) -> bool {
    let mut is_collided = false;
    let mut active_tetris_piece_grid_points =
        gameworld
            .entities
            .iter_mut()
            .find(|e| {
                e.as_any().is::<TetrisPiece>()
            })
            .unwrap()
            .as_any()
            .downcast_ref::<TetrisPiece>()
            .unwrap()
            .piece_type_shape
            .clone();
    let lowest_grid_point =
        active_tetris_piece_grid_points
            .iter()
            .map(|gp| {
                entities::idx_to_grid_point(
                    gp.point_idx,
                )
                .y
            })
            .max()
            .unwrap()
            .clone();
    let tetris_grid = gameworld
        .entities
        .iter_mut()
        .find(|e| e.as_any().is::<TetrisGrid>())
        .unwrap()
        .as_any_mut()
        .downcast_mut::<TetrisGrid>()
        .unwrap();
    // check if the active piece is about to collide
    for gp in
        active_tetris_piece_grid_points.iter_mut()
    {
        // this 'or' must short circuit else index out of bounds
        if lowest_grid_point >= 13
            || tetris_grid.matrix
                [gp.point_idx as usize + 10]
                .point_state
                == TetrisGridPointState::Active
        {
            is_collided = true;
            break;
        }
    }
    // if collision has occured then update the tetris grid
    if is_collided {
        for gp in active_tetris_piece_grid_points
            .iter_mut()
        {
            // tetris_grid.matrix[gp.point_idx as usize].point_state = TetrisGridPointState::Active;
            tetris_grid.matrix
                [gp.point_idx as usize]
                .switch_grid_point_state(
                    TetrisGridPointState::Active,
                );
        }
        // remove active tetris piece entity so new one can be created
        gameworld.entities.retain(|entity| {
            !entity.as_any().is::<TetrisPiece>()
        });
    }
    is_collided
}
async fn manage_music(gameworld: &mut GameWorld) {
    // Use fold to find both intro and looped tracks in one pass
    if let (
        Some(intro_music),
        Some(looped_music),
    ) = gameworld
        .get_entities_by_type::<MusicTrack>()
        .iter_mut()
        .fold((None, None), |mut acc, track| {
            // Check if it's an intro track we care about
            match track.name {
                TrackName::BubbleManIntro
                | TrackName::WoodManIntro
                | TrackName::FlashManIntro
                | TrackName::CrashManIntro => {
                    if acc.0.is_none() {
                        // Only take the first intro found
                        acc.0 = Some(track);
                    }
                }
                // Check if it's a corresponding loop track we care about
                TrackName::BubbleManLoop
                | TrackName::WoodManLoop
                | TrackName::FlashManLoop
                | TrackName::CrashManLoop => {
                    if acc.1.is_none() {
                        // Only take the first loop found
                        acc.1 = Some(track);
                    }
                }
                // Add other relevant loop tracks if necessary (e.g., for robots without intros)
                TrackName::AirMan
                | TrackName::QuickMan
                | TrackName::HeatMan
                | TrackName::MetalMan
                | TrackName::Wily1_2
                | TrackName::RustMan => {
                    if acc.1.is_none() {
                        // If no intro, this might be the main loop track
                        acc.1 = Some(track);
                    }
                }
                _ => {} // Ignore other tracks like Title, StageSelect, etc.
            }
            acc // Return the updated accumulator tuple
        })
    {
        // for each type of intro, check if it is ended and start playing looped music
        match (
            &intro_music.name,
            intro_music.elapsed,
        ) {
            (
                TrackName::BubbleManIntro,
                14.5..,
            )
            | (TrackName::WoodManIntro, 10.2..)
            | (
                TrackName::FlashManIntro,
                28.3..,
            )
            | (
                TrackName::CrashManIntro,
                16.8..,
            ) => looped_music.play(),
            _ => {}
        }
    }
}
