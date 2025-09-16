//
// gui::utils
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(super) struct ScreenSpaceManager {
    selection_queue: UiArea,
    timeline: UiArea,
    phoneme_table: UiArea,
    data_info: UiArea,
}
// ----------------------------------------------------------------------------
pub(super) struct UiArea {
    pub pos: (f32, f32),
    pub size: (f32, f32),
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
impl ScreenSpaceManager {
    // ------------------------------------------------------------------------
    #[rustfmt::skip]
    pub fn new(winsize: (f32, f32), queue_height: Option<f32>) -> ScreenSpaceManager {
        let (width, height) = winsize;

        let menubar_height = 19.0;
        let timeline_height = 220.0;

        let queue_area_height = queue_height.unwrap_or(212.0);
        let queue_area_min_height = 150.0;
        let queue_area_max_height = (height - timeline_height - menubar_height - 20.0).max(150.0);

        let queue_area_height = queue_area_height.clamp(queue_area_min_height, queue_area_max_height);

        let middle_area_size = (width, height - (timeline_height + menubar_height + queue_area_height));

        // areas - top timeline
        let timeline_area = ((0.0, menubar_height), (width, timeline_height)).into();

        // areas - middle [phoneme table | info area]
        let phoneme_area = ((0.0, menubar_height + timeline_height), (width * 0.5, middle_area_size.1)).into();
        let info_area = ((width * 0.5, menubar_height + timeline_height), (width * 0.5, middle_area_size.1)).into();

        // areas - bottom selection queue (resizeable)
        let selection_area = ((0.0, height - queue_area_height), (width, queue_area_height)).into();

        ScreenSpaceManager {
            selection_queue: selection_area,
            timeline: timeline_area,
            phoneme_table: phoneme_area,
            data_info: info_area,
        }
    }
    // ------------------------------------------------------------------------
    pub fn selection_queue(&self) -> &UiArea {
        &self.selection_queue
    }
    // ------------------------------------------------------------------------
    pub fn timeline(&self) -> &UiArea {
        &self.timeline
    }
    // ------------------------------------------------------------------------
    pub fn phoneme_table(&self) -> &UiArea {
        &self.phoneme_table
    }
    // ------------------------------------------------------------------------
    pub fn data_info(&self) -> &UiArea {
        &self.data_info
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<((f32, f32), (f32, f32))> for UiArea {
    fn from(v: ((f32, f32), (f32, f32))) -> UiArea {
        UiArea {
            pos: v.0,
            size: v.1,
        }
    }
}
// ----------------------------------------------------------------------------
