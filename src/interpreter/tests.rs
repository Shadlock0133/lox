use crate::errors::ResolveError;

use super::*;

#[track_caller]
fn run(x: &str) -> String {
    let tokens = crate::tokenizer::Tokenizer::new(x)
        .filter(|t| t.as_ref().map(|t| !t.can_skip()).unwrap_or(true))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut ast = crate::parser::Parser::new(tokens).parse().unwrap();

    let mut output = vec![];
    let mut interpreter = Interpreter::new(&mut output);

    crate::resolver::Resolver::new(&mut interpreter.locals)
        .resolve(&mut ast)
        .unwrap();

    interpreter.interpret(&mut ast).unwrap();
    drop(interpreter);
    String::from_utf8(output).unwrap()
}

#[track_caller]
fn interpreter_error(x: &str) -> RuntimeError {
    let tokens = crate::tokenizer::Tokenizer::new(x)
        .filter(|t| t.as_ref().map(|t| !t.can_skip()).unwrap_or(true))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut ast = crate::parser::Parser::new(tokens).parse().unwrap();

    let mut output = vec![];
    let mut interpreter = Interpreter::new(&mut output);
    crate::resolver::Resolver::new(&mut interpreter.locals)
        .resolve(&mut ast)
        .unwrap();

    let error = interpreter.interpret(&mut ast).unwrap_err().into_error();
    error
}

#[track_caller]
fn resolver_error(x: &str) -> ResolveError {
    let tokens = crate::tokenizer::Tokenizer::new(x)
        .filter(|t| t.as_ref().map(|t| !t.can_skip()).unwrap_or(true))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut ast = crate::parser::Parser::new(tokens).parse().unwrap();

    let mut output = vec![];
    let mut interpreter = Interpreter::new(&mut output);
    crate::resolver::Resolver::new(&mut interpreter.locals)
        .resolve(&mut ast)
        .unwrap_err()
}

#[test]
fn simple() {
    assert_eq!(run("print \"one\";"), "one\n");
    assert_eq!(run("print true;"), "true\n");
    assert_eq!(run("print 1 + 2;"), "3\n");
    assert_eq!(run("var a = 1; print a;"), "1\n");
    assert_eq!(run("var a = 1; print a = 2;"), "2\n");
    assert_eq!(run("class Foo {} print Foo;"), "Foo\n");
    assert_eq!(run("class Foo {} print Foo();"), "Foo instance\n");
}

#[test]
fn scopes() {
    assert_eq!(
        run("var a = 1;
            var b = 1;
            print a;
            print b;
            {
                var a = 2;
                b = 2;
                print a;
                print b;
            }
            print a;
            print b;"),
        "1\n1\n2\n2\n1\n2\n"
    );

    assert_eq!(
        run("var a = 1;
            {
                fun print_a() {
                    print a;
                }
                print_a(); // 1

                a = 2;
                print_a(); // 2

                var a = 3;
                print_a(); // 2, not 3
            }"),
        "1\n2\n2\n"
    );
}

#[test]
fn scope_error() {
    assert_eq!(
        resolver_error(
            "{
                var a = 1;
                var a = 2;
            }"
        )
        .to_string(),
        "[line 3:21] Resolve Error at 'a': Already variable with this name in this scope."
    );

    assert_eq!(
        resolver_error(
            "var a = 1;
            {
                var a = a + 2;
                print a;
            }"
        )
        .to_string(),
        "[line 3:25] Resolve Error at 'a': Can't read local variable in its own initializer."
    );
}

#[test]
fn closure_error() {
    assert_eq!(
        interpreter_error(
            "fun foo() {
                var a = 1;
            }
            foo();
            print a;"
        )
        .to_string(),
        "[line 5:19] Runtime Error at 'a': Undefined variable 'a'."
    )
}

#[test]
fn factorial() {
    assert_eq!(
        run("fun fact(a) {
                if (a <= 1)
                    return 1;
                else
                    return a * fact(a - 1);
            }
            print fact(20);"),
        ((1..=20).map(|x| x as f64).product::<f64>().to_string() + "\n")
    );
}
