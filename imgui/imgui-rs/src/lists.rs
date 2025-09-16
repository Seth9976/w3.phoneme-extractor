use sys;

use std::marker::PhantomData;
use std::os::raw::c_char;

use super::{ImString, ImStr, Ui};

#[must_use]
pub struct ListBox<'ui, 'p> {
    label: Option<&'p ImStr>,
    items:  &'p [&'p ImStr],
    selected: &'p mut i32,
    autowidth: bool,
    height_in_items: i32,
    _phantom: PhantomData<&'ui Ui<'ui>>,
}

impl<'ui, 'p> ListBox<'ui, 'p> {
    pub fn new(_: &Ui<'ui>, items: &'p [&'p ImStr], current_item: &'p mut i32) -> Self {
        ListBox {
            label: None,
            items,
            selected: current_item,
            autowidth: false,
            height_in_items: 4,
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub fn label(mut self, label: &'p ImStr) -> Self {
        self.autowidth = false;
        self.label = Some(label);
        self
    }
    #[inline]
    pub fn autowidth(mut self) -> Self {
        self.autowidth = true;
        self
    }
    #[inline]
    pub fn height_in_items(mut self, items: i32) -> Self {
        self.height_in_items = items;
        self
    }
    pub fn build(self) -> bool {
        let items_inner: Vec<*const c_char> = self.items.iter().map(|item| item.as_ptr()).collect();
        unsafe {
            if self.autowidth {
                sys::igPushItemWidth(-1.0);
            }
            let empty = ImString::new("##empty");

            let result = sys::igListBox(self.label.unwrap_or(&empty).as_ptr(),
                                 self.selected,
                                 items_inner.as_ptr() as *mut *const c_char,
                                 items_inner.len() as i32,
                                 self.height_in_items);
            if self.autowidth {
                sys::igPopItemWidth();
            }
            result
        }
    }
}
