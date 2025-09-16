//
// input form field validator
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub type StringValidator = Box<dyn Fn(&str) -> Result<(), ImString>>;
pub type IntValidator = Box<dyn Fn(i32) -> Result<(), ImString>>;
pub type FloatValidator = Box<dyn Fn(f32) -> Result<(), ImString>>;
// ----------------------------------------------------------------------------
pub fn is_nonempty() -> StringValidator {
    Box::new(|input: &str| {
        if input.is_empty() {
            Err(ImString::new("must not be empty"))
        } else {
            Ok(())
        }
    })
}
// ----------------------------------------------------------------------------
pub fn is_ascii() -> StringValidator {
    Box::new(|input: &str| {
        if input.is_ascii() {
            Ok(())
        } else {
            Err(ImString::new("must not contain non-ascii characters"))
        }
    })
}
// ----------------------------------------------------------------------------
pub fn length(len: usize) -> StringValidator {
    Box::new(move |input: &str| {
        if input.chars().count() != len {
            Err(ImString::new(format!("must be {} characters long", len)))
        } else {
            Ok(())
        }
    })
}
// ----------------------------------------------------------------------------
pub fn min_length(len: usize) -> StringValidator {
    Box::new(move |input: &str| {
        let chars = input.chars().count();
        if chars >= 1 && chars < len {
            Err(ImString::new(format!(
                "must be at least {} characters long",
                len
            )))
        } else {
            Ok(())
        }
    })
}
// ----------------------------------------------------------------------------
pub fn max_length(len: usize) -> StringValidator {
    Box::new(move |input: &str| {
        if input.chars().count() > len {
            Err(ImString::new(format!(
                "must be at most {} characters long",
                len
            )))
        } else {
            Ok(())
        }
    })
}
// ----------------------------------------------------------------------------
pub fn chars(regexpr: &'static str, valid_characters_msg: &'static str) -> StringValidator {
    use regex::Regex;
    let re = Regex::new(regexpr).unwrap();

    Box::new(move |input: &str| {
        if re.is_match(input) {
            Ok(())
        } else {
            Err(ImString::new(format!(
                "must contain only following characters: {}",
                valid_characters_msg
            )))
        }
    })
}
// ----------------------------------------------------------------------------
pub fn is_hhmm() -> StringValidator {
    Box::new(move |input: &str| {
        if input.chars().count() == 5 {
            let (hour, minutes) = input.split_at(2);
            let (separator, minutes) = minutes.split_at(1);

            if let Ok(hour) = hour.parse::<u8>() {
                if let Ok(minutes) = minutes.parse::<u8>() {
                    if separator == ":" && hour <= 23 && minutes <= 59 {
                        return Ok(());
                    }
                }
            }
        }
        Err(ImString::new("time must be given as HH:mm"))
    })
}
// ----------------------------------------------------------------------------
pub fn is_hhmmss() -> StringValidator {
    Box::new(move |input: &str| {
        if input.chars().count() == 8 {
            let (hour, min_sec) = input.split_at(2);
            let (minutes, seconds) = min_sec.split_at(3);
            let (separator1, minutes) = minutes.split_at(1);
            let (separator2, seconds) = seconds.split_at(1);

            if hour.parse::<u8>().is_ok() {
                if let Ok(minutes) = minutes.parse::<u8>() {
                    if let Ok(seconds) = seconds.parse::<u8>() {
                        if separator1 == ":" && separator2 == ":" && minutes <= 59 && seconds <= 59
                        {
                            return Ok(());
                        }
                    }
                }
            }
        }
        Err(ImString::new("time must be given as HH:mm:ss"))
    })
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui::ImString;
// ----------------------------------------------------------------------------
