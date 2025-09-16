//
// input form fields
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub trait Field {
    fn valid(&self) -> bool;
    fn changed(&self) -> bool;
    fn error(&self) -> Result<&(), &ImString>;
    fn id(&self) -> &str;
    fn value(&self) -> FieldValue;
    fn draw(&mut self, ui: &Ui<'_>) -> Option<FieldValue>;
    fn validate(&mut self) -> &mut dyn Field;
    fn set_value(&mut self, new_value: FieldValue) -> Result<(), String>;
    fn set_error_msg(&mut self, error: String);
    fn reset(&mut self);
    fn reset_changed(&mut self);
}
// ----------------------------------------------------------------------------
pub struct TextField {
    id: ImString,
    label: Option<ImString>,
    value: ImString,
    label_width: f32,
    width: f32,
    buf: ImString,
    changed: bool,
    error: Result<(), ImString>,
    validator: Vec<validator::StringValidator>,
}
// ----------------------------------------------------------------------------
pub struct FloatField {
    id: ImString,
    label: Option<ImString>,
    value: f32,
    label_width: f32,
    width: f32,
    buf: f32,
    changed: bool,
    error: Result<(), ImString>,
    validator: Vec<validator::FloatValidator>,
}
// ----------------------------------------------------------------------------
pub struct IntField {
    id: ImString,
    label: Option<ImString>,
    value: i32,
    label_width: f32,
    width: f32,
    buf: i32,
    changed: bool,
    error: Result<(), ImString>,
    validator: Vec<validator::IntValidator>,
}
// ----------------------------------------------------------------------------
pub struct BoolField {
    id: ImString,
    label: Option<ImString>,
    value: bool,
    label_width: f32,
    width: f32,
    changed: bool,
}
// ----------------------------------------------------------------------------
pub enum FieldValue<'a> {
    Str(&'a str),
    Bool(bool),
    Int(i32),
    Float(f32),
    // None,
}
// ----------------------------------------------------------------------------
pub enum FieldAction<'a, A> {
    ValueUpdate(FieldValue<'a>),
    Custom(A),
}
// ----------------------------------------------------------------------------
pub mod validator;
// ----------------------------------------------------------------------------
impl<'a> FieldValue<'a> {
    // ------------------------------------------------------------------------
    pub fn as_str(&self) -> Result<&'a str, String> {
        match *self {
            FieldValue::Str(value) => Ok(value),
            _ => Err(String::from("value not a string")),
        }
    }
    // ------------------------------------------------------------------------
    pub fn as_f32(&self) -> Result<f32, String> {
        match *self {
            FieldValue::Float(value) => Ok(value),
            _ => Err(String::from("value not a float")),
        }
    }
    // ------------------------------------------------------------------------
    pub fn as_i32(&self) -> Result<i32, String> {
        match *self {
            FieldValue::Int(value) => Ok(value),
            _ => Err(String::from("value not an int")),
        }
    }
    // ------------------------------------------------------------------------
    pub fn as_bool(&self) -> Result<bool, String> {
        match *self {
            FieldValue::Bool(value) => Ok(value),
            _ => Err(String::from("value not a bool")),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui::{ImString, Ui};

const COL_RED: (f32, f32, f32, f32) = (1.0, 0.0, 0.0, 1.0);
const COL_DEFAULT: (f32, f32, f32, f32) = (255.0, 255.0, 255.0, 1.0);
// ----------------------------------------------------------------------------
// macros
// ----------------------------------------------------------------------------
macro_rules! draw_field {
    ($field: expr, $ui: ident, $field_widget: expr) => {{
        use imgui::ImGuiCol;

        let colvars = match $field.error {
            Err(_) => [(ImGuiCol::Text, COL_RED)],
            Ok(_) => [(ImGuiCol::Text, COL_DEFAULT)],
        };

        let mut changed = false;
        $ui.with_color_vars(&colvars, || {
            if let Some(ref label) = $field.label {
                $ui.text(label);
                $ui.same_line($field.label_width);
            }
            #[allow(clippy::redundant_closure_call)]
            $ui.with_item_width($field.width, || {
                changed = $field_widget();
            });
        });
        if let Err(ref err) = $field.error {
            if $ui.is_item_hovered() {
                $ui.tooltip(|| {
                    $ui.text(err);
                });
            }
        }
        if changed {
            $field.changed = true;
            Some($field.buffered())
        } else {
            None
        }
    }};
}
// ----------------------------------------------------------------------------
// Text Field
// ----------------------------------------------------------------------------
impl TextField {
    // ------------------------------------------------------------------------
    fn init_buf<T: Into<String>>(txt: Option<T>) -> (ImString, ImString) {
        let value = if let Some(txt) = txt {
            let mut txt = ImString::new(txt);
            let capa = txt.capacity();
            if capa < 256 {
                txt.reserve(256 - capa);
            }
            txt
        } else {
            ImString::with_capacity(256)
        };
        (value.clone(), value)
    }
    // ------------------------------------------------------------------------
    pub fn new<S: Into<String>, T: Into<String>>(id: S, txt: Option<T>) -> TextField {
        let (value, buf) = Self::init_buf(txt);

        TextField {
            id: ImString::new(id.into()),
            label: None,
            value,
            label_width: 0.0,
            width: -1.0,
            buf,
            changed: false,
            error: Ok(()),
            validator: Vec::new(),
        }
    }
    // ------------------------------------------------------------------------
    pub fn new_with_label<S: Into<String>>(id: S, label: S, txt: Option<S>) -> TextField {
        let (value, buf) = Self::init_buf(txt);

        TextField {
            id: ImString::new(id.into()),
            label: Some(ImString::new(label.into())),
            value,
            label_width: 0.0,
            width: -1.0,
            buf,
            changed: false,
            error: Ok(()),
            validator: Vec::new(),
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = Some(ImString::new(label.into()));
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn adjust_width(&mut self, width: f32) -> &mut Self {
        self.width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_validators(mut self, v: Vec<validator::StringValidator>) -> Self {
        self.validator = v;
        self
    }
    // ------------------------------------------------------------------------
    pub fn custom_validate<F, S>(&mut self, validate: F) -> &mut Self
    where
        F: FnOnce(&str) -> Result<(), S>,
        S: Into<ImString>,
    {
        if self.error.is_ok() {
            self.error = validate(self.value.as_ref()).map_err(Into::into);
        }
        self
    }
    // ------------------------------------------------------------------------
    fn buffered(&self) -> FieldValue {
        FieldValue::Str(self.buf.as_ref())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Field for TextField {
    // ------------------------------------------------------------------------
    #[inline]
    fn id(&self) -> &str {
        self.id.as_ref()
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn valid(&self) -> bool {
        self.error.is_ok()
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn changed(&self) -> bool {
        self.changed
    }
    // ------------------------------------------------------------------------
    fn error(&self) -> Result<&(), &ImString> {
        self.error.as_ref()
    }
    // ------------------------------------------------------------------------
    fn value(&self) -> FieldValue {
        FieldValue::Str(self.value.as_ref())
    }
    // ------------------------------------------------------------------------
    fn draw(&mut self, ui: &Ui<'_>) -> Option<FieldValue> {
        draw_field!(self, ui, || ui
            .input_text(&self.id, &mut self.buf)
            .allow_tab_input(false)
            .build())
    }
    // ------------------------------------------------------------------------
    fn validate(&mut self) -> &mut dyn Field {
        self.error = Ok(());

        for validator in &self.validator {
            self.error = validator(self.buf.as_ref());
            if self.error.is_err() {
                break;
            }
        }
        self
    }
    // ------------------------------------------------------------------------
    fn reset(&mut self) {
        let (value, buf) = Self::init_buf::<String>(None);
        self.value = value;
        self.buf = buf;
        self.changed = false;
    }
    // ------------------------------------------------------------------------
    fn reset_changed(&mut self) {
        self.changed = false;
    }
    // ------------------------------------------------------------------------
    fn set_value(&mut self, new_value: FieldValue) -> Result<(), String> {
        match new_value {
            FieldValue::Str(value) => self.value = ImString::new(value),
            _ => {
                return Err(String::from(
                    "setting non string value in input field not supported",
                ))
            }
        }
        Ok(())
    }
    // ------------------------------------------------------------------------
    fn set_error_msg(&mut self, error: String) {
        self.error = Err(ImString::new(error));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// Float Field
// ----------------------------------------------------------------------------
impl FloatField {
    // ------------------------------------------------------------------------
    pub fn new<S: Into<String>>(id: S, value: Option<f32>) -> FloatField {
        FloatField {
            id: ImString::new(id.into()),
            label: None,
            value: value.unwrap_or(0.0),
            label_width: 0.0,
            width: -1.0,
            buf: value.unwrap_or(0.0),
            changed: false,
            error: Ok(()),
            validator: Vec::new(),
        }
    }
    // ------------------------------------------------------------------------
    pub fn new_with_label<S: Into<String>>(id: S, label: S, value: Option<f32>) -> FloatField {
        FloatField {
            id: ImString::new(id.into()),
            label: Some(ImString::new(label.into())),
            value: value.unwrap_or(0.0),
            buf: value.unwrap_or(0.0),
            label_width: 0.0,
            width: -1.0,
            changed: false,
            error: Ok(()),
            validator: Vec::new(),
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = Some(ImString::new(label.into()));
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn adjust_width(&mut self, width: f32) -> &mut Self {
        self.width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_validators(mut self, v: Vec<validator::FloatValidator>) -> Self {
        self.validator = v;
        self
    }
    // ------------------------------------------------------------------------
    pub fn custom_validate<F, S>(&mut self, validate: F) -> &mut Self
    where
        F: FnOnce(f32) -> Result<(), S>,
        S: Into<ImString>,
    {
        if self.error.is_ok() {
            self.error = validate(self.value).map_err(Into::into);
        }
        self
    }
    // ------------------------------------------------------------------------
    fn buffered(&self) -> FieldValue {
        FieldValue::Float(self.buf)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Field for FloatField {
    // ------------------------------------------------------------------------
    #[inline]
    fn id(&self) -> &str {
        self.id.as_ref()
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn valid(&self) -> bool {
        self.error.is_ok()
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn changed(&self) -> bool {
        self.changed
    }
    // ------------------------------------------------------------------------
    fn error(&self) -> Result<&(), &ImString> {
        self.error.as_ref()
    }
    // ------------------------------------------------------------------------
    fn value(&self) -> FieldValue {
        FieldValue::Float(self.value)
    }
    // ------------------------------------------------------------------------
    fn draw(&mut self, ui: &Ui<'_>) -> Option<FieldValue> {
        draw_field!(self, ui, || ui
            .input_float(&self.id, &mut self.buf)
            .allow_tab_input(false)
            .build())
    }
    // ------------------------------------------------------------------------
    fn validate(&mut self) -> &mut dyn Field {
        self.error = Ok(());

        for validator in &self.validator {
            self.error = validator(self.buf);
            if self.error.is_err() {
                break;
            }
        }
        self
    }
    // ------------------------------------------------------------------------
    fn reset(&mut self) {
        self.value = 0.0;
        self.buf = 0.0;
        self.changed = false;
    }
    // ------------------------------------------------------------------------
    fn reset_changed(&mut self) {
        self.changed = false;
    }
    // ------------------------------------------------------------------------
    fn set_value(&mut self, new_value: FieldValue) -> Result<(), String> {
        match new_value {
            FieldValue::Float(value) => self.value = value,
            _ => {
                return Err(String::from(
                    "setting non float value in input field not supported",
                ))
            }
        }
        Ok(())
    }
    // ------------------------------------------------------------------------
    fn set_error_msg(&mut self, error: String) {
        self.error = Err(ImString::new(error));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// Int Field
// ----------------------------------------------------------------------------
impl IntField {
    // ------------------------------------------------------------------------
    pub fn new<S: Into<String>>(id: S, value: Option<i32>) -> IntField {
        IntField {
            id: ImString::new(id.into()),
            label: None,
            value: value.unwrap_or(0),
            label_width: 0.0,
            width: -1.0,
            buf: value.unwrap_or(0),
            changed: false,
            error: Ok(()),
            validator: Vec::new(),
        }
    }
    // ------------------------------------------------------------------------
    pub fn new_with_label<S: Into<String>>(id: S, label: S, value: Option<i32>) -> IntField {
        IntField {
            id: ImString::new(id.into()),
            label: Some(ImString::new(label.into())),
            value: value.unwrap_or(0),
            buf: value.unwrap_or(0),
            label_width: 0.0,
            width: -1.0,
            changed: false,
            error: Ok(()),
            validator: Vec::new(),
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = Some(ImString::new(label.into()));
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn adjust_width(&mut self, width: f32) -> &mut Self {
        self.width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_validators(mut self, v: Vec<validator::IntValidator>) -> Self {
        self.validator = v;
        self
    }
    // ------------------------------------------------------------------------
    pub fn custom_validate<F, S>(&mut self, validate: F) -> &mut Self
    where
        F: FnOnce(i32) -> Result<(), S>,
        S: Into<ImString>,
    {
        if self.error.is_ok() {
            self.error = validate(self.value).map_err(Into::into);
        }
        self
    }
    // ------------------------------------------------------------------------
    fn buffered(&self) -> FieldValue {
        FieldValue::Int(self.buf)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Field for IntField {
    // ------------------------------------------------------------------------
    #[inline]
    fn id(&self) -> &str {
        self.id.as_ref()
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn valid(&self) -> bool {
        self.error.is_ok()
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn changed(&self) -> bool {
        self.changed
    }
    // ------------------------------------------------------------------------
    fn error(&self) -> Result<&(), &ImString> {
        self.error.as_ref()
    }
    // ------------------------------------------------------------------------
    fn value(&self) -> FieldValue {
        FieldValue::Int(self.value)
    }
    // ------------------------------------------------------------------------
    fn draw(&mut self, ui: &Ui<'_>) -> Option<FieldValue> {
        draw_field!(self, ui, || ui
            .input_int(&self.id, &mut self.buf)
            .step(1)
            .step_fast(25)
            .build())
    }
    // ------------------------------------------------------------------------
    fn validate(&mut self) -> &mut dyn Field {
        self.error = Ok(());

        for validator in &self.validator {
            self.error = validator(self.buf);
            if self.error.is_err() {
                break;
            }
        }
        self
    }
    // ------------------------------------------------------------------------
    fn reset(&mut self) {
        self.value = 0;
        self.buf = 0;
        self.changed = false;
    }
    // ------------------------------------------------------------------------
    fn reset_changed(&mut self) {
        self.changed = false;
    }
    // ------------------------------------------------------------------------
    fn set_value(&mut self, new_value: FieldValue) -> Result<(), String> {
        match new_value {
            FieldValue::Int(value) => self.value = value,
            _ => {
                return Err(String::from(
                    "setting non integral value in input field not supported",
                ))
            }
        }
        Ok(())
    }
    // ------------------------------------------------------------------------
    fn set_error_msg(&mut self, error: String) {
        self.error = Err(ImString::new(error));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// Bool Field
// ----------------------------------------------------------------------------
impl BoolField {
    // ------------------------------------------------------------------------
    pub fn new<S: Into<String>>(id: S, value: bool) -> BoolField {
        BoolField {
            id: ImString::new(id.into()),
            label: None,
            value,
            label_width: 0.0,
            width: -1.0,
            changed: false,
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = Some(ImString::new(label.into()));
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn adjust_width(&mut self, width: f32) -> &mut Self {
        self.width = width;
        self
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Field for BoolField {
    // ------------------------------------------------------------------------
    #[inline]
    fn id(&self) -> &str {
        self.id.as_ref()
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn valid(&self) -> bool {
        true
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn changed(&self) -> bool {
        self.changed
    }
    // ------------------------------------------------------------------------
    fn error(&self) -> Result<&(), &ImString> {
        Ok(&())
    }
    // ------------------------------------------------------------------------
    fn value(&self) -> FieldValue {
        FieldValue::Bool(self.value)
    }
    // ------------------------------------------------------------------------
    fn draw(&mut self, ui: &Ui<'_>) -> Option<FieldValue> {
        let mut value = self.value;
        let mut changed = false;
        if let Some(ref label) = self.label {
            ui.text(label);
            ui.same_line(self.label_width);
        }

        if ui.checkbox(&self.id, &mut value) {
            changed = true;
        }
        if changed {
            self.changed = true;
            Some(FieldValue::Bool(value))
        } else {
            None
        }
    }
    // ------------------------------------------------------------------------
    fn validate(&mut self) -> &mut dyn Field {
        self
    }
    // ------------------------------------------------------------------------
    fn reset(&mut self) {
        self.value = false;
        self.changed = false;
    }
    // ------------------------------------------------------------------------
    fn reset_changed(&mut self) {
        self.changed = false;
    }
    // ------------------------------------------------------------------------
    fn set_value(&mut self, new_value: FieldValue) -> Result<(), String> {
        match new_value {
            FieldValue::Bool(value) => self.value = value,
            _ => {
                return Err(String::from(
                    "setting non bool value in input field not supported",
                ))
            }
        }
        Ok(())
    }
    // ------------------------------------------------------------------------
    fn set_error_msg(&mut self, _error: String) {}
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
