#[cfg_attr(not(rust_analyzer), gpui_macros::derive_inspector_reflection)]
trait Transform: Clone {
    /// Doubles the value
    fn double(self) -> Self;

    /// Triples the value
    fn triple(self) -> Self;

    /// Increments the value by one
    ///
    /// This method has a default implementation
    fn increment(self) -> Self {
        self.add_one()
    }

    /// Quadruples the value by doubling twice
    fn quadruple(self) -> Self {
        self.double().double()
    }

    #[allow(dead_code)]
    fn add(&self, other: &Self) -> Self;
    #[allow(dead_code)]
    fn set_value(&mut self, value: i32);
    #[allow(dead_code)]
    fn get_value(&self) -> i32;

    /// Adds one to the value
    fn add_one(self) -> Self;
}

#[derive(Debug, Clone, PartialEq)]
struct Number(i32);

impl Transform for Number {
    fn double(self) -> Self {
        Number(self.0 * 2)
    }

    fn triple(self) -> Self {
        Number(self.0 * 3)
    }

    fn add(&self, other: &Self) -> Self {
        Number(self.0 + other.0)
    }

    fn set_value(&mut self, value: i32) {
        self.0 = value;
    }

    fn get_value(&self) -> i32 {
        self.0
    }

    fn add_one(self) -> Self {
        Number(self.0 + 1)
    }
}

#[test]
fn derives_inspector_reflection() {
    use transform_reflection::*;

    let methods = methods::<Number>();
    assert_eq!(methods.len(), 5);
    let method_names: Vec<_> = methods.iter().map(|method| method.name).collect();
    assert!(method_names.contains(&"double"));
    assert!(method_names.contains(&"triple"));
    assert!(method_names.contains(&"increment"));
    assert!(method_names.contains(&"quadruple"));
    assert!(method_names.contains(&"add_one"));

    let number = Number(5);
    assert_eq!(
        find_method::<Number>("double")
            .unwrap()
            .invoke(number.clone()),
        Number(10)
    );
    assert_eq!(
        find_method::<Number>("triple")
            .unwrap()
            .invoke(number.clone()),
        Number(15)
    );
    assert_eq!(
        find_method::<Number>("increment")
            .unwrap()
            .invoke(number.clone()),
        Number(6)
    );
    assert_eq!(
        find_method::<Number>("quadruple").unwrap().invoke(number),
        Number(20)
    );
    assert!(find_method::<Number>("nonexistent").is_none());

    let number = Number(10);
    let result = find_method::<Number>("double")
        .map(|method| method.invoke(number))
        .and_then(|number| find_method::<Number>("increment").map(|method| method.invoke(number)))
        .and_then(|number| find_method::<Number>("triple").map(|method| method.invoke(number)));
    assert_eq!(result, Some(Number(63)));

    assert_eq!(
        find_method::<Number>("double").unwrap().documentation,
        Some("Doubles the value")
    );
    assert_eq!(
        find_method::<Number>("triple").unwrap().documentation,
        Some("Triples the value")
    );
    assert_eq!(
        find_method::<Number>("increment").unwrap().documentation,
        Some("Increments the value by one\n\nThis method has a default implementation")
    );
    assert_eq!(
        find_method::<Number>("quadruple").unwrap().documentation,
        Some("Quadruples the value by doubling twice")
    );
    assert_eq!(
        find_method::<Number>("add_one").unwrap().documentation,
        Some("Adds one to the value")
    );
}
