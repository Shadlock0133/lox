use crate::clox::value::{ObjString, Value};

use super::Table;

#[test]
fn simple_test() {
    let hola = ObjString::new("hola".into());
    let mundo = Value::string("mundo".into());

    let hello = ObjString::new("hello".into());
    let world = Value::string("world".into());
    let za_warudo = Value::string("za warudo".into());

    let mut table = Table::default();

    assert!(table.insert(hola.clone(), mundo.clone()));
    assert!(table.insert(hello.clone(), world.clone()));

    assert!(table
        .get(&hello)
        .unwrap()
        .clone()
        .into_obj_string()
        .is_some());
    assert_eq!(table.get(&hello), Some(&world));
    assert_eq!(table.get(&hola), Some(&mundo));

    assert!(!table.insert(hello.clone(), za_warudo.clone()));
    assert_eq!(table.get(&hello), Some(&za_warudo));

    *table.get_mut(&hola).unwrap() = Value::bool(true);

    assert_eq!(table.get(&hola), Some(Value::bool(true)).as_ref());

    assert_eq!(table.remove(&hello).as_ref(), Some(&za_warudo));
    assert!(table.get(&hello).is_none());

    assert_eq!(table.get(&hola), Some(Value::bool(true)).as_ref());

    let other: Vec<_> = (0..10)
        .map(|i| ObjString::new(format!("filler{}", i)))
        .collect();

    for k in &other {
        table.insert(k.clone(), Value::nil());
    }

    for k in &other {
        table.remove(k);
    }

    for k in &other {
        table.insert(k.clone(), Value::nil());
    }
}

#[test]
fn many_inserts() {
    let mut table = Table::default();

    let n = 10000;

    for i in 0..n {
        table.insert(ObjString::new(i.to_string()), Value::Number(i as _));
    }

    for i in 0..n {
        assert_eq!(
            table.get(&ObjString::new(i.to_string())),
            Some(&Value::Number(i as _))
        );
    }

    for i in 0..n {
        match table.get_mut(&ObjString::new(i.to_string())) {
            Some(Value::Number(n)) => *n += 1.0,
            _ => unreachable!(),
        }
    }

    for i in 0..n {
        assert_eq!(
            table.get(&ObjString::new(i.to_string())),
            Some(&Value::Number((i + 1) as _))
        );
    }
}
