//
// base matrix container
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
//#[derive(Default)]
pub struct Matrix2D<T: Default> {
    pub width: usize,
    pub height: usize,
    pub data: Vec<T>,
}
// ----------------------------------------------------------------------------
pub struct DebugMatrix2D<'a, T, U>
where
    T: 'a + Default + fmt::Display,
    U: 'a + fmt::Display,
{
    matrix: &'a Matrix2D<T>,
    col_names: &'a [U],
    row_names: &'a [U],
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
const MAX_MATRIX_SIZE: usize = 500;

use std::fmt;
use std::ops::{Index, IndexMut};
impl<T: Default + Clone> Matrix2D<T> {
    // ------------------------------------------------------------------------
    pub fn new(width: usize, height: usize) -> Matrix2D<T> {
        Self::new_with_default(width, height, T::default())
    }
    // ------------------------------------------------------------------------
    pub fn new_with_default(width: usize, height: usize, default: T) -> Matrix2D<T> {
        if !(1..=MAX_MATRIX_SIZE).contains(&width) || !(1..=MAX_MATRIX_SIZE).contains(&height) {
            fatal!(
                "width and height for alignment matrix must be within [1;{}]. got: {}x{}",
                MAX_MATRIX_SIZE,
                width,
                height
            );
        }
        Matrix2D {
            width,
            height,
            data: vec![default; height * width],
        }
    }
    // ------------------------------------------------------------------------
    pub fn add_row(&mut self, elements: &mut Vec<T>) -> Result<(), String> {
        if elements.len() == self.width {
            self.data.append(elements);
            self.height += 1;
            Ok(())
        } else {
            Err(format!(
                "expected {} elements. found: {}",
                self.width,
                elements.len()
            ))
        }
    }
    // ------------------------------------------------------------------------
    pub fn row(&self, r: usize) -> impl Iterator<Item = &T> {
        assert!((r + 1) * self.width <= self.data.len());
        self.data[r* self.width..(r+1)*self.width].iter()
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<T: Default> Index<(usize, usize)> for Matrix2D<T> {
    type Output = T;
    // ------------------------------------------------------------------------
    fn index(&self, index: (usize, usize)) -> &T {
        assert!(index.0 < self.width, "x out of bound!");
        assert!(index.1 < self.height, "y out of bound!");

        &self.data[index.1 * self.width + index.0]
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<T: Default> IndexMut<(usize, usize)> for Matrix2D<T> {
    // ------------------------------------------------------------------------
    fn index_mut(&mut self, index: (usize, usize)) -> &mut T {
        &mut self.data[index.1 * self.width + index.0]
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
impl<'a, T, U> DebugMatrix2D<'a, T, U>
where
    T: Default + fmt::Display,
    U: 'a + fmt::Display,
{
    // ------------------------------------------------------------------------
    pub fn new(
        matrix: &'a Matrix2D<T>,
        col_names: &'a [U],
        row_names: &'a [U],
    ) -> DebugMatrix2D<'a, T, U> {
        DebugMatrix2D {
            matrix,
            col_names,
            row_names,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<T, U> fmt::Display for DebugMatrix2D<'_, T, U>
where
    T: Default + fmt::Display,
    U: fmt::Display,
{
    // ------------------------------------------------------------------------
    #[allow(unused_must_use)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // scorematrix has extra first row/col without phonemes -> pad names
        let colname_padding = self.matrix.width - self.col_names.len();
        let rowname_padding = self.matrix.height - self.row_names.len();

        write!(f, "\n    ");
        for w in 0..self.matrix.width {
            if w >= colname_padding {
                write!(f, "|{: ^4}", &self.col_names[w - colname_padding]);
            } else {
                write!(f, "|    ");
            }
        }

        for h in 0..self.matrix.height {
            if h >= rowname_padding {
                write!(f, "\n{: ^4}", &self.row_names[h - rowname_padding]);
            } else {
                write!(f, "\n    ");
            }

            for w in 0..self.matrix.width {
                let s = &self.matrix.data[h * self.matrix.width + w];
                write!(f, "|{: ^4}", s);
            }
        }
        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
