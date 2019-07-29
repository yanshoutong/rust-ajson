use path::{Path, Query};
use reader;
use reader::ByteReader;
use std::str;
use value::Value;

pub fn new_path(v: &[u8]) -> Path {
    let (path, _) = parse_path(v);
    path
}

fn parser_query_value(v: &[u8]) -> (Value, usize) {
    println!("parse query value {:?}", String::from_utf8_lossy(v));
    let mut reader = reader::RefReader::new(v);
    while let Some(b) = reader.next() {
        let value = match b {
            b't' => {
                reader.read_boolean_value();
                Value::Boolean(true)
            }
            b'f' => {
                reader.read_boolean_value();
                Value::Boolean(false)
            }
            b'n' => {
                reader.read_null_value();
                Value::Null
            }
            b'"' => {
                // println!("======");
                let (start, end) = reader.read_str_value();
                let raw = reader.slice(start + 1, end - 1);
                let s = String::from_utf8_lossy(raw).to_string();
                Value::String(s)
                // Value::Null
            }
            b'0'...b'9' | b'-' => {
                let (start, end) = reader.read_number_value();
                let raw = reader.slice(start, end);
                // TODO
                let f = str::from_utf8(raw).unwrap().parse().unwrap();
                Value::Number(f)
            }
            _ => Value::NotExists,
        };

        return (value, reader.position());
    }

    (Value::NotExists, reader.position())
}

fn parse_query<'a>(v: &'a [u8]) -> (Query<'a>, usize) {
    println!("parse query {:?}", v);
    println!("parse query str {:?}", String::from_utf8_lossy(v));

    let mut depth = 1;
    let mut reader = reader::RefReader::new(v);
    let mut q = Query::empty();

    let (key, offset) = parse_path(reader.tail(v));
    // println!("find path in query {:?}, {}", key, offset);
    q.set_key(key);
    reader.forward(offset);


    q.set_on(true);
    let op_start = reader.position();
    let mut op_exist = false;
    let mut op_end = op_start;
    while let Some(b) = reader.peek() {
        match b {
            b'!' | b'=' | b'<' | b'>' | b'%' => {
                if depth == 1 {
                    op_exist = true;
                    op_end = reader.position();
                }
            }
            b']' | b')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            b' ' => continue,
            _ => {
                let (val, offset) = parser_query_value(reader.tail(v));
                q.set_val(val);
                reader.forward(offset);
                break;
            }
        };


        reader.next();
    }

    // println!("op {} {}", op_start, op_end);

    if op_exist {
        let op = String::from_utf8_lossy(reader.slice(op_start, op_end)).to_string();
        q.set_op(op);
    }

    
    match reader.next() {
        Some(b'#') => q.set_all(true),
        Some(_) => reader.back(1),
        None => (),
    }


    (q, reader.position())
}

fn parse_path<'a>(v: &'a [u8]) -> (Path<'a>, usize) {
    println!("parse path {:?}", String::from_utf8_lossy(v));
    let mut current_path = Path::new();
    let mut reader = reader::RefReader::new(v);
    let mut end = 0;
    let mut part_exsit = true;
    let mut depth = 0;
    current_path.set_ok(true);

    while let Some(b) = reader.peek() {
        match b {
            b'\\' => {
                reader.next();
            }
            b']' | b')' => {
                if depth > 0 {
                    depth -= 0;
                }
                if depth == 0 {
                    end = reader.position() - 1;
                    break;
                }
            }
            b'!' | b'=' | b'<' | b'>' | b'%' => {
                if depth == 0 && reader.position() == 0 {
                    part_exsit = false;
                }
                
                break;
            }
            b'.' => {
                end = reader.position() - 1;
                current_path.set_more(true);
                reader.next();
                let (next, offset) = parse_path(reader.tail(v));
                current_path.set_next(next);
                reader.forward(offset);
                break;
            }
            b'*' | b'?' => current_path.set_wild(true),
            b'#' => current_path.set_arrch(true),
            b'[' | b'(' => {
                depth += 1;
                if depth == 1 && current_path.arrch {
                    reader.next();
                    let (query, offset) = parse_query(reader.tail(v));
                    current_path.set_q(query);
                    reader.forward(offset);
                }
            }
            _ => (),
        };

        end = reader.position();
        reader.next();
    }
    if part_exsit {
        // println!("set path part {}", end);
        current_path.set_part(reader.head(v, end));
    } else {
        current_path.set_ok(false);
    }

    (current_path, reader.position())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fn_parse_path() {
        let v = r#"name"#.as_bytes();
        let p = parse_path(&v);
        println!("{:?}", p);
        println!("======================");

        let v = r#"#(last=="Murphy")#.first"#.as_bytes();
        let p = parse_path(&v);
        println!("{:?}", p);
        println!("======================");

        let v = r#"friends.#(first!%"D*")#.last"#.as_bytes();
        let p = parse_path(&v);
        println!("{:?}", p);
        println!("======================");

        let v = r#"c?ildren.0"#.as_bytes();
        let p = parse_path(&v);
        println!("{:?}", p);
        println!("======================");

        let v = r#"#(sub_item>7)#.title"#.as_bytes();
        let p = parse_path(&v);
        println!("{:?}", p);
        println!("======================");
    }

    #[test]
    fn test_fn_parse_query() {
        let v = "first)".as_bytes();
        let (q, _) = parse_query(&v);
        println!("{:?}", q);
        println!("======================");

        let v = "first)#".as_bytes();
        let (q, _) = parse_query(&v);
        println!("{:?}", q);
        println!("======================");

        let v = r#"first="name")"#.as_bytes();
        let (q, _) = parse_query(&v);
        println!("{:?}", q);
        println!("======================");

        let v = r#"nets.#(=="ig"))"#.as_bytes();
        let (q, _) = parse_query(&v);
        println!("{:?}", q);
        println!("======================");

        let v = r#"nets.#(=="ig"))#"#.as_bytes();
        let (q, _) = parse_query(&v);
        println!("{:?}", q);
        println!("======================");

        let v = r#"=="ig")"#.as_bytes();
        let (q, _) = parse_query(&v);
        println!("{:?}", q);
        println!("======================");

        let v = r#"first=)"#.as_bytes();
        let (q, _) = parse_query(&v);
        println!("{:?}", q);
        println!("======================");

        let v = r#"sub_item>7)#.title"#.as_bytes();
        let (q, _) = parse_query(&v);
        println!("{:?}", q);
        println!("======================");
    }
}