//! Temporary bridge from compatibility State transitions to the client stack.
//! Concrete menu tick/render functions move separately in stages 6/7.

use crate::{
    core::display_stack::DisplayStack,
    screen::display::{Display, ParentPolicy},
};

use super::State;

#[cfg(test)]
pub(super) fn observe_world(world: &crate::world::World, game: &mut crate::core::game::Game) {
    // Runtime handles already share state; no per-tick snapshot propagation.
    let _ = (world, game);
}

/// Clock and updater state survive through Game's shared handles. Readiness changes
/// only after the requested destination crosses the logical update boundary.
pub(super) fn begin_game_tick(
    displays: &mut DisplayStack<State>,
    game: &mut crate::core::game::Game,
    focused: bool,
) {
    crate::core::updater::begin_display_tick_observed(displays, |event, state| {
        if event == crate::core::display_stack::LifecycleEvent::Init {
            match state {
                State::Title { .. } => game.enter_title(),
                State::Playing { world, .. } => {
                    game.attach_world_runtime(world.shared_clock(), world.shared_updater());
                }
                _ => {}
            }
        }
    });
    if !matches!(displays.current(), Some(State::Playing { .. })) {
        game.tick_menu(focused);
    }
}

impl Display for State {
    fn parent_policy(&self) -> ParentPolicy {
        match self {
            // Playing is a compatibility host, not a Java Display. Detaching
            // prevents menus from retaining a world or returning into an old game.
            State::Title { .. } | State::Playing { .. } => ParentPolicy::Root,
            _ => ParentPolicy::Inherit,
        }
    }
}

impl crate::core::updater::DisplayHost for State {
    fn update_child_displays(&mut self) {
        if let State::Playing { world, .. } = self {
            world.begin_display_tick();
        }
    }
}

/// Translate the legacy destination into either a push, return to an existing
/// parent, or in-place edit. Returning must not reconstruct/reset the parent.
/// Audio remains in the existing input dispatch; exit() must not play it twice.
pub(super) fn navigate(displays: &mut DisplayStack<State>, next: State) {
    let current = displays.current_id().expect("application screen");
    let kind = std::mem::discriminant(&next);
    if std::mem::discriminant(displays.current().unwrap()) == kind {
        *displays.current_mut().unwrap() = next;
        return;
    }
    let mut parent = displays.parent_id(current);
    let mut depth = 1;
    while let Some(id) = parent {
        if std::mem::discriminant(displays.get(id).unwrap()) == kind {
            displays.exit(depth);
            return;
        }
        parent = displays.parent_id(id);
        depth += 1;
    }
    displays.open(next);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn title() -> DisplayStack<State> {
        let mut stack = DisplayStack::default();
        stack.open(State::title(1, 73));
        stack.update();
        stack
    }

    #[test]
    fn lifecycle_observer_initializes_intermediate_world_before_final_title() {
        let mut world = crate::world::registry_tests::tiny_world();
        world.tick(&crate::io::Input::default());
        let record = crate::worlds::WorldRecord {
            name: "intermediate".into(),
            seed: 0,
            spec: crate::world::WorldSpec::new(128, 0, 0),
            mode: crate::world::GameMode::Survival,
            score_minutes: 20,
            directory: Default::default(),
        };
        let mut stack = title();
        let mut game = crate::core::game::Game::default();
        game.enter_world(crate::core::game::ClockSnapshot::new(99, 0, true));
        stack.open(State::Playing {
            world: Box::new(world),
            record,
        });
        stack.open(State::title(0, 0));
        begin_game_tick(&mut stack, &mut game, false);
        assert!(!game.ready());
        // The intermediate world is already dropped, but its time and paused
        // state were adopted before Title's calendar prelude.
        assert_eq!(
            (game.game_time(), game.calendar_tick(), game.paused()),
            (1, 1, false)
        );
    }

    #[test]
    fn real_world_tick_save_load_and_queued_title_child_preserve_session_state() {
        use crate::world::{GameMode, World, WorldSpec, registry_tests::tiny_world};
        let world = tiny_world();
        let original_clock = world.shared_clock();
        let record = crate::worlds::WorldRecord {
            name: "lifecycle fixture".into(),
            seed: 0,
            spec: WorldSpec::new(128, 0, 0),
            mode: GameMode::Survival,
            score_minutes: 20,
            directory: Default::default(),
        };
        let mut stack = title();
        let mut game = crate::core::game::Game::default();
        stack.open(State::Playing {
            world: Box::new(world),
            record: record.clone(),
        });
        begin_game_tick(&mut stack, &mut game, true);
        let Some(State::Playing { world, .. }) = stack.current_mut() else {
            panic!()
        };
        world.tick_after_display(true, &crate::io::Input::default());
        assert!(game.shares_clock(&original_clock));
        // Read BEFORE the Updater pause-gate handoff: there is no time copy.
        assert_eq!(game.game_time(), 1);
        observe_world(world, &mut game);
        assert_eq!(game.game_time(), 1);
        assert!(!game.paused());
        assert!(game.ready());
        let saved = world.to_save_string().unwrap();
        world.tick_after_display(false, &crate::io::Input::default());
        observe_world(world, &mut game);
        assert_eq!((game.game_time(), game.calendar_tick()), (1, 1));

        stack.open(State::title(0, 0));
        stack.open(State::Options {
            menu: super::super::build_options_menu(
                &crate::resource::localization::Localization::load("en-us", &[]),
                &crate::io::Settings::default(),
                2,
            ),
        });
        assert!(game.ready());
        begin_game_tick(&mut stack, &mut game, false);
        assert!(matches!(
            stack.current(),
            Some(State::Options { menu }) if menu.selection == 2
        ));
        assert!(!game.ready());
        assert_eq!(
            (game.game_time(), game.calendar_tick(), game.paused()),
            (1, 2, false)
        );
        begin_game_tick(&mut stack, &mut game, true);
        assert_eq!(
            (game.game_time(), game.calendar_tick(), game.paused()),
            (2, 3, true)
        );

        let loaded = World::from_save_string(&saved).unwrap();
        let loaded_clock = loaded.shared_clock();
        assert!(!loaded_clock.same_instance(&original_clock));
        stack.open(State::Playing {
            world: Box::new(loaded),
            record,
        });
        begin_game_tick(&mut stack, &mut game, true);
        assert!(game.ready());
        assert_eq!(
            (game.game_time(), game.calendar_tick(), game.paused()),
            (1, 0, false)
        );
        assert!(game.shares_clock(&loaded_clock));
        original_clock.set_game_time(99);
        assert_eq!(game.game_time(), 1);
        loaded_clock.advance_focused(true);
        assert_eq!(game.game_time(), 2);
    }

    #[test]
    fn global_time_continues_through_title_children_and_focus_loss() {
        let mut stack = title();
        let mut game = crate::core::game::Game::default();
        begin_game_tick(&mut stack, &mut game, true);
        navigate(
            &mut stack,
            State::Options {
                menu: super::super::build_options_menu(
                    &crate::resource::localization::Localization::load("en-us", &[]),
                    &crate::io::Settings::default(),
                    0,
                ),
            },
        );
        begin_game_tick(&mut stack, &mut game, true);
        begin_game_tick(&mut stack, &mut game, false);
        assert_eq!(game.game_time(), 2);
        navigate(&mut stack, State::title(0, 0));
        begin_game_tick(&mut stack, &mut game, true);
        assert_eq!(game.game_time(), 3);
        assert!(!game.ready());
    }

    #[test]
    fn queued_title_changes_readiness_only_at_lifecycle_boundary() {
        let mut stack = title();
        let mut game = crate::core::game::Game::default();
        // Model a ready outgoing session without allocating a generated world.
        game.enter_world(crate::core::game::ClockSnapshot::new(120, 0, true));
        stack.open(State::title(0, 0));
        assert!(game.ready());
        assert_eq!(game.game_time(), 120);
        begin_game_tick(&mut stack, &mut game, false);
        assert!(!game.ready());
        assert_eq!(game.game_time(), 120);
        begin_game_tick(&mut stack, &mut game, true);
        assert_eq!(game.game_time(), 121);
    }

    #[test]
    fn title_options_child_returns_preserve_both_parent_selections() {
        let mut stack = title();
        let title_id = stack.current_id();
        let options_state = State::Options {
            menu: super::super::build_options_menu(
                &crate::resource::localization::Localization::load("en-us", &[]),
                &crate::io::Settings::default(),
                6,
            ),
        };
        navigate(&mut stack, options_state);
        assert_eq!(stack.current_id(), title_id);
        stack.update();
        let options_id = stack.current_id();
        navigate(
            &mut stack,
            State::Languages {
                menu: super::super::build_languages_menu(
                    &crate::resource::localization::Localization::load("en-us", &[]),
                    4,
                ),
            },
        );
        stack.update();
        navigate(
            &mut stack,
            State::Options {
                menu: super::super::build_options_menu(
                    &crate::resource::localization::Localization::load("en-us", &[]),
                    &crate::io::Settings::default(),
                    0,
                ),
            },
        );
        stack.update();
        assert_eq!(stack.current_id(), options_id);
        assert!(matches!(
            stack.current(),
            Some(State::Options { menu }) if menu.selection == 6
        ));
        navigate(&mut stack, State::title(0, 0));
        stack.update();
        assert_eq!(stack.current_id(), title_id);
        assert!(matches!(
            stack.current(),
            Some(State::Title {
                menu,
                ticks: 73,
                ..
            }) if menu.selection == 3 // Options is entry 3 after checking+blank+Play
        ));
    }

    #[test]
    fn editing_controls_does_not_push_another_screen_or_lose_parent() {
        let mut stack = title();
        let options = State::Options {
            menu: super::super::build_options_menu(
                &crate::resource::localization::Localization::load("en-us", &[]),
                &crate::io::Settings::default(),
                4,
            ),
        };
        navigate(&mut stack, options);
        stack.update();
        let options_id = stack.current_id();
        navigate(
            &mut stack,
            State::Controls {
                menu: super::super::build_controls_menu(
                    &crate::resource::localization::Localization::load("en-us", &[]),
                    true,
                    2,
                ),
                capture: None,
                keyboard: true,
            },
        );
        stack.update();
        let controls_id = stack.current_id();
        navigate(
            &mut stack,
            State::Controls {
                menu: super::super::build_controls_menu(
                    &crate::resource::localization::Localization::load("en-us", &[]),
                    true,
                    2,
                ),
                capture: Some((2, true)),
                keyboard: true,
            },
        );
        stack.update();
        assert_eq!(stack.current_id(), controls_id);
        assert_eq!(stack.parent_id(controls_id.unwrap()), options_id);
        assert!(matches!(
            stack.current(),
            Some(State::Controls {
                capture: Some((2, true)),
                ..
            })
        ));
    }

    #[test]
    fn real_title_and_options_input_use_stack_and_freeze_title_while_covered() {
        let mut stack = title();
        let mut settings = crate::io::Settings::default();
        let Some(State::Title { menu, ticks, .. }) = stack.current_mut() else {
            panic!()
        };
        let transition = super::super::tick_title_menu_event(
            menu,
            ticks,
            &crate::io::Input {
                select: true,
                ..Default::default()
            },
            0,
            &crate::resource::localization::Localization::load("en-us", &[]),
        )
        .0;
        assert!(matches!(transition, super::super::Transition::Options));
        navigate(
            &mut stack,
            State::Options {
                menu: super::super::build_options_menu(
                    &crate::resource::localization::Localization::load("en-us", &[]),
                    &settings,
                    0,
                ),
            },
        );
        stack.update();
        let Some(State::Options { menu }) = stack.current_mut() else {
            panic!()
        };
        let (transition, _) = super::super::tick_options_menu(
            menu,
            &crate::io::Input {
                exit: true,
                ..Default::default()
            },
            &mut settings,
        );
        assert!(matches!(transition, super::super::Transition::Title));
        navigate(&mut stack, State::title(0, 0));
        stack.update();
        assert!(matches!(
            stack.current(),
            Some(State::Title {
                menu,
                ticks: 74,
                ..
            }) if menu.selection == 3
        ));
    }
}
