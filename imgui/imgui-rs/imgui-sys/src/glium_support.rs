use glium::vertex::{Attribute, AttributeType, Vertex, VertexFormat};
use std::borrow::Cow;
use std::os::raw::c_float;

use super::{ImDrawVert, ImVec2, ImVec4};

#[cfg(feature = "glium")]
unsafe impl Attribute for ImVec2 {
    fn get_type() -> AttributeType { <(c_float, c_float) as Attribute>::get_type() }
}

#[cfg(feature = "glium")]
unsafe impl Attribute for ImVec4 {
    fn get_type() -> AttributeType {
        <(c_float, c_float, c_float, c_float) as Attribute>::get_type()
    }
}

#[cfg(feature = "glium")]
#[allow(clippy::unneeded_field_pattern)]
impl Vertex for ImDrawVert {
    fn build_bindings() -> VertexFormat {
        Cow::Owned(vec![
            (
                "pos".into(),
                memoffset::offset_of!(ImDrawVert, pos),
                <ImVec2 as Attribute>::get_type(),
                false
            ),
            (
                "uv".into(),
                memoffset::offset_of!(ImDrawVert, uv),
                <ImVec2 as Attribute>::get_type(),
                false
            ),
            (
                "col".into(),
                memoffset::offset_of!(ImDrawVert, col),
                AttributeType::U8U8U8U8,
                false
            ),
        ])
    }
}
