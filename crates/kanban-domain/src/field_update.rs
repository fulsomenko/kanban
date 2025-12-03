/// Represents a field update operation for partial updates
///
/// This type provides a clear, three-state pattern for updating optional fields:
/// - `NoChange`: Field keeps its existing value
/// - `Set(value)`: Field is updated to the provided value
/// - `Clear`: Field is cleared (set to None)
///
/// # Example
///
/// ```
/// use kanban_domain::FieldUpdate;
///
/// let title_update = FieldUpdate::Set("New Title".to_string());
/// let description_update = FieldUpdate::Clear;
/// let priority_update = FieldUpdate::NoChange;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldUpdate<T> {
    /// Do not modify this field (keep existing value)
    NoChange,
    /// Set the field to the provided value
    Set(T),
    /// Clear the field (set to None)
    Clear,
}

impl<T> Default for FieldUpdate<T> {
    fn default() -> Self {
        FieldUpdate::NoChange
    }
}

impl<T> FieldUpdate<T> {
    /// Apply this update to an optional field
    ///
    /// # Example
    ///
    /// ```
    /// use kanban_domain::FieldUpdate;
    ///
    /// let mut field = Some("old value".to_string());
    /// let update = FieldUpdate::Set("new value".to_string());
    /// update.apply_to(&mut field);
    /// assert_eq!(field, Some("new value".to_string()));
    ///
    /// let clear = FieldUpdate::Clear;
    /// clear.apply_to(&mut field);
    /// assert_eq!(field, None);
    /// ```
    pub fn apply_to(self, field: &mut Option<T>) {
        match self {
            FieldUpdate::NoChange => {}
            FieldUpdate::Set(value) => *field = Some(value),
            FieldUpdate::Clear => *field = None,
        }
    }

    /// Check if this represents a change (not NoChange)
    pub fn is_change(&self) -> bool {
        !matches!(self, FieldUpdate::NoChange)
    }
}

impl<T> From<Option<T>> for FieldUpdate<T> {
    /// Convert Option<T> to FieldUpdate<T>
    /// - Some(value) becomes Set(value)
    /// - None becomes Clear
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(value) => FieldUpdate::Set(value),
            None => FieldUpdate::Clear,
        }
    }
}
