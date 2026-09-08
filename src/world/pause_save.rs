//! Flat save-format bridge. Keep the old eight keys readable; new saves also
//! preserve the actual ancestor frames instead of guessing their selections.

use minicraft_client::screen::pause::{PausePage, PauseSession, PauseView};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Keep serialization out of the client core. Remote derives are exclusively
// owned by this save adapter; the client still depends only on common.
#[derive(Serialize, Deserialize)]
#[serde(remote = "PausePage")]
enum SavedPage {
    Main,
    Options,
    Achievements,
    Quests,
    QuestSeriesInfo,
    QuestSeriesViewer,
    QuestDetail,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "PauseView")]
struct SavedView {
    #[serde(with = "SavedPage")]
    page: PausePage,
    selection: usize,
    confirm: bool,
    quest_tab: usize,
    quest_group: usize,
    quest_item: usize,
    quest_series_selection: usize,
}

#[derive(Serialize, Deserialize)]
struct Frame(#[serde(with = "SavedView")] PauseView);

mod frames {
    use super::*;
    pub(super) fn serialize<S: Serializer>(
        value: &[PauseView],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value
            .iter()
            .copied()
            .map(Frame)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<PauseView>, D::Error> {
        Ok(Vec::<Frame>::deserialize(deserializer)?
            .into_iter()
            .map(|frame| frame.0)
            .collect())
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct SavedPause {
    paused: bool,
    pause_selection: usize,
    #[serde(with = "SavedPage")]
    pause_page: PausePage,
    pause_confirm: bool,
    pause_quest_tab: usize,
    pause_quest_group: usize,
    pause_quest_item: usize,
    pause_quest_series_selection: usize,
    #[serde(with = "frames", skip_serializing_if = "Vec::is_empty")]
    pause_frames: Vec<PauseView>,
}

pub(super) fn serialize<S: Serializer>(
    pause: &PauseSession,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let view = pause.view();
    SavedPause {
        paused: pause.is_open(),
        pause_selection: view.selection,
        pause_page: view.page,
        pause_confirm: view.confirm,
        pause_quest_tab: view.quest_tab,
        pause_quest_group: view.quest_group,
        pause_quest_item: view.quest_item,
        pause_quest_series_selection: view.quest_series_selection,
        pause_frames: pause.frames(),
    }
    .serialize(serializer)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<PauseSession, D::Error> {
    let saved = SavedPause::deserialize(deserializer)?;
    if !saved.paused {
        return Ok(PauseSession::default());
    }
    if !saved.pause_frames.is_empty() {
        // Only these seven pages and the confirmation popup can form a path.
        if saved.pause_frames.len() > 6 {
            return Err(serde::de::Error::custom("pause frame path is too deep"));
        }
        let first = saved.pause_frames[0];
        if first.page != PausePage::Main
            || first.confirm
            || saved.pause_frames.windows(2).any(|pair| {
                let (parent, child) = (pair[0], pair[1]);
                parent.confirm
                    || if child.confirm {
                        parent.page != PausePage::Main || child.page != PausePage::Main
                    } else {
                        !matches!(
                            (parent.page, child.page),
                            (
                                PausePage::Main,
                                PausePage::Options | PausePage::Achievements | PausePage::Quests
                            ) | (PausePage::Quests, PausePage::QuestSeriesInfo)
                                | (PausePage::QuestSeriesInfo, PausePage::QuestSeriesViewer)
                                | (PausePage::QuestSeriesViewer, PausePage::QuestDetail)
                        )
                    }
            })
        {
            return Err(serde::de::Error::custom("invalid pause parent path"));
        }
        return Ok(PauseSession::from_frames(&saved.pause_frames));
    }
    let view = PauseView {
        page: saved.pause_page,
        selection: saved.pause_selection,
        confirm: saved.pause_confirm,
        quest_tab: saved.pause_quest_tab,
        quest_group: saved.pause_quest_group,
        quest_item: saved.pause_quest_item,
        quest_series_selection: saved.pause_quest_series_selection,
    };
    // Old files had no parent identity. Reconstruct the known page hierarchy;
    // an exact old parent cursor that was never saved cannot be recovered.
    let mut frames = vec![PauseView {
        selection: match view.page {
            PausePage::Options => 1,
            PausePage::Achievements => 2,
            PausePage::Quests
            | PausePage::QuestSeriesInfo
            | PausePage::QuestSeriesViewer
            | PausePage::QuestDetail => 3,
            // Clamped to the final Main Menu entry once World options are known.
            PausePage::Main if view.confirm => usize::MAX,
            _ => 0,
        },
        ..PauseView::default()
    }];
    if matches!(
        view.page,
        PausePage::QuestSeriesInfo | PausePage::QuestSeriesViewer | PausePage::QuestDetail
    ) {
        frames.push(PauseView {
            page: PausePage::Quests,
            selection: view.quest_series_selection,
            confirm: false,
            ..view
        });
    }
    if matches!(
        view.page,
        PausePage::QuestSeriesViewer | PausePage::QuestDetail
    ) {
        frames.push(PauseView {
            page: PausePage::QuestSeriesInfo,
            selection: 0,
            confirm: false,
            ..view
        });
    }
    if view.page == PausePage::QuestDetail {
        frames.push(PauseView {
            page: PausePage::QuestSeriesViewer,
            selection: view.quest_item,
            confirm: false,
            ..view
        });
    }
    if view.page == PausePage::Main && !view.confirm {
        frames[0] = view;
    } else {
        frames.push(view);
    }
    Ok(PauseSession::from_frames(&frames))
}

pub(super) fn normalize(pause: &mut PauseSession, quests_enabled: bool) {
    let mut frames = pause.frames();
    if let Some(root) = frames.first_mut() {
        root.selection = root.selection.min(if quests_enabled { 5 } else { 4 });
        *pause = PauseSession::from_frames(&frames);
    }
}
