use super::TextEdit;

impl TextEdit {
    /// The height in points a person dragged the field to by its resize grip, as a browser keeps
    /// a dragged `<textarea>`'s size; `None` until dragged. Replacing the text keeps it.
    pub fn dragged_height(&self) -> Option<f32> {
        self.dragged_height
    }

    pub fn set_dragged_height(&mut self, height: Option<f32>) {
        self.dragged_height = height.filter(|h| h.is_finite());
    }
}
