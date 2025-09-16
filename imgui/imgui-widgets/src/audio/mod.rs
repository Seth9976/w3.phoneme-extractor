use imgui::{Ui, ImStr};

use super::AudioWidgets;

use self::waveform::WaveForm;
use self::timeline::{Timeline, Block};

mod waveform;
mod timeline;

impl AudioWidgets for Ui<'_> {
    fn audio(&self) -> Widgets {
        Widgets::new(self)
    }
}

pub struct Widgets<'ui> {
    ui: &'ui Ui<'ui>,
}

impl<'ui> Widgets<'ui> {
    fn new(ui: &'ui Ui) -> Widgets<'ui> {
        Widgets {
            ui
        }
    }

    pub fn waveform<'p>(&self, label: &'p ImStr, values: &'p[i16], visible_range: &'p mut (usize, usize))
        -> WaveForm<'ui, 'p>
    {
        WaveForm::new(self.ui, label, values, visible_range)
    }

    pub fn timeline<'p>(&self, label: &'p ImStr, duration: f32, hover_pos: &'p mut f32) -> Timeline<'ui, 'p>
    {
        Timeline::new(self.ui, label, duration, hover_pos)
    }

    pub fn timeline_block<'p>(
        &self,
        id: &'p ImStr,
        label: &'p ImStr,
        timeframe: (f32, f32),
        timeframe_clipping: (f32, f32))
        -> Block<'ui, 'p>
    {
        Block::new(self.ui, id, label, timeframe, timeframe_clipping)
    }
}
