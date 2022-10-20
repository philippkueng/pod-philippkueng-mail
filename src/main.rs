use std::collections::HashMap;
use std::io::{BufReader, self, Write};

use bencode_rs::Value;
use bencode_rs as bc;

use serde::{Deserialize, Serialize};
use serde_json::Value as SerdeValue;

use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

fn get_string(val: &bc::Value, key: &str) -> Option<String> {
    match val {
        bc::Value::Map(hm) => {
            match hm.get(&Value::from(key)) {
                Some(Value::Str(s)) =>
                Some(String::from(s)),
                _ => None
            }
        },
        _ => None
    }
}

fn insert(mut m: HashMap<Value,Value>, k: &str, v: &str) -> HashMap<Value,Value> {
    m.insert(Value::from(k), Value::from(v));
    m
}

fn write_describe_map() {
    let namespace = HashMap::new();
    let mut namespace = insert(namespace, "name", "pod.philippkueng.mail");
    let mut vars = Vec::new();
    let var_map = HashMap::new();
    let var_map = insert(var_map, "name", "send");
    vars.push(Value::from(var_map));
    namespace.insert(Value::from("vars"),Value::List(vars));
    let describe_map = HashMap::new();
    let mut describe_map = insert(describe_map, "format", "json");
    let namespaces = vec![Value::from(namespace)];
    let namespaces = Value::List(namespaces);
    describe_map.insert(Value::from("namespaces"), namespaces);
    let describe_map = Value::from(describe_map);
    let bencode = describe_map.to_bencode();
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(bencode.as_bytes()).unwrap();
    handle.flush().unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
struct Payload {
    host: String,
    username: String,
    password: String,
    subject: String,
    from: String,
    to: String,
    text: String
}

fn handle_incoming(val: bc::Value) {
    let op = get_string(&val, "op").unwrap();
    match &op[..] {
        "describe" => {
            write_describe_map()
        },
        "invoke" => {
            let var = get_string(&val, "var").unwrap();
            match &var[..] {
                "pod.philippkueng.mail/send" => {
                    let id = get_string(&val, "id").unwrap();
                    let reply = HashMap::new();

                    let args = get_string(&val, "args").unwrap();
                    let mut parsed_args: Vec<SerdeValue> = serde_json::from_str(&args).unwrap();

                    let payload: SerdeValue = serde_json::from_value(parsed_args.remove(0)).unwrap();
                    let payload_as_string = payload.as_str().unwrap();
                    let parsed_payload: Payload = serde_json::from_str(&payload_as_string).unwrap();

                    // --- sending email

                    let email = Message::builder()
                        .from(parsed_payload.from.parse().unwrap())
                        .to(parsed_payload.to.parse().unwrap())
                        .subject(parsed_payload.subject)
                        .body(String::from(parsed_payload.text))
                        .unwrap();

                    let creds = Credentials::new(parsed_payload.username, parsed_payload.password);

                    // Open a remote connection to gmail
                    let host = parsed_payload.host;
                    let mailer = SmtpTransport::relay(&host)
                        .unwrap()
                        .credentials(creds)
                        .build();

                    // Send the email
                    let mut message: &str = "";
                    match mailer.send(&email) {
                        Ok(_) => message = "Email sent successfully!",
                        Err(_e) => message = "Could not send email: {:?}"
                    }

                    // --- end sending email

                    let value = serde_json::json!({
                        "message": message
                    });
                    let value = value.to_string();

                    let reply = insert(reply, "value", &value);
                    let reply = insert(reply, "id", &id);

                    let bencode = Value::from(reply).to_bencode();
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();
                    handle.write_all(bencode.as_bytes()).unwrap();
                    handle.flush().unwrap();
                },
                _ => panic!("Unknown var: {}", var)
            };
        },
        _ => panic!("Unknown op: {}", op)
    }
}

fn main() {
    loop {
        let mut reader = BufReader::new(io::stdin());
        let val = bc::parse_bencode(&mut reader);
        match val {
            Ok(Some(val)) => {
                handle_incoming(val)
            },
            Ok(None) => {
                return
            }
            Err(bc::BencodeError::Eof()) => {
                return
            },
            Err(e) => panic!("Error: {}", e)
        }
    }
}
