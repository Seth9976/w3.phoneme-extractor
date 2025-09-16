use std;
use std::os::raw::c_int;
use sys;

use super::ImString;

pub struct FontAtlas {
    // keep ownership of font configs
    configs: Vec<FontConfig>,

    atlas: *mut sys::ImFontAtlas,
}

pub enum Font {
    Default,
    FromFile(ImString),
}

/// Id of added font used to reference specific font on font changes
pub struct FontId(usize);

pub struct FontConfig {
    file: Option<ImString>,
    pixel_size: f32,
    conf: sys::ImFontConfig,
    glyphrange: Option<GlyphRange>,
}

pub struct GlyphRange(Vec<u16>);

impl Default for FontConfig {

    fn default() -> FontConfig {
        let conf = FontConfig {
            file: None,
            pixel_size: 15.0,
            conf: sys::ImFontConfig::default(),
            glyphrange: None,
        };
        conf.set_oversampling(1, 1)
    }

}

impl FontAtlas {

    pub(super) fn new(atlas: *mut sys::ImFontAtlas) -> FontAtlas {
        FontAtlas {
            configs: Vec::default(),
            atlas,
        }
    }

    pub(super) fn add_font(&mut self, font: Font, mut config: FontConfig) -> FontId {
        match font {
            Font::Default => unsafe {
                sys::ImFontAtlas_AddFontDefault(
                    self.atlas,
                    config.font_config(),
                );
            },
            Font::FromFile(path) => unsafe {
                sys::ImFontAtlas_AddFontFromFileTTF(
                    self.atlas,
                    path.as_ptr(),
                    config.pixel_size(),
                    config.font_config(),
                    config.glyphrange().map_or(std::ptr::null(), |r| &r[0])
                );
                // keep ownership of ImString
                config.file = Some(path);
            }
        }
        self.configs.push(config);
        FontId(self.configs.len() - 1)
    }

    pub(super) fn font(&self, id: &FontId) -> Option<*mut sys::ImFont> {
        let len = unsafe { sys::ImFontAtlas_Fonts_size(self.atlas) };
        if len > id.0 as i32 {
            unsafe {
                Some(sys::ImFontAtlas_Fonts_index(self.atlas, id.0 as c_int))
            }
        } else {
            None
        }
    }
}

impl GlyphRange {
    pub fn new(start: u16, end: u16) -> Self {
        let mut r = GlyphRange(Vec::new());
        r.0.push(start);
        r.0.push(end);
        r
    }

    #[inline]
    pub fn add(mut self, start: u16, end: u16) -> Self {
        self.0.push(start);
        self.0.push(end);
        self
    }
}

impl FontConfig {
    pub fn new(fontsize: f32) -> Self {
        FontConfig {
            file: None,
            pixel_size: fontsize,
            conf: sys::ImFontConfig::default(),
            glyphrange: None,
        }
    }

    #[inline]
    pub fn set_oversampling(mut self, horizontal: c_int, vertical: c_int) -> Self {
        self.conf.oversample_h = horizontal;
        self.conf.oversample_v = vertical;
        self
    }

    #[inline]
    pub fn set_glyphrange(mut self, mut range: GlyphRange) -> Self {
        range.0.push(0);

        self.glyphrange = Some(range);
        self
    }

    #[inline]
    pub fn set_offset<T: Into<sys::ImVec2>>(mut self, offset: T) -> Self {
        self.conf.glyph_offset = offset.into();
        self
    }

    #[inline]
    pub fn set_extra_spacing<T: Into<sys::ImVec2>>(mut self, spacing: T) -> Self {
        self.conf.glyph_extra_spacing = spacing.into();
        self
    }

    #[inline]
    pub(super) fn pixel_size(&self) -> f32 {
        self.pixel_size
    }

    #[inline]
    pub(super) fn font_config(&self) -> &sys::ImFontConfig {
        &self.conf
    }

    #[inline]
    pub(super) fn glyphrange(&self) -> Option<&Vec<u16>> {
        self.glyphrange.as_ref().map(|r| &r.0)
    }
}
