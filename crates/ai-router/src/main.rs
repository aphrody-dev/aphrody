//! ai-router models [auto|auto:local|auto:free|auto:cheap|<provider>/<model>] [--json]
//! ai-router chat [--model <policy or id>] <prompt...>      (prompt from stdin when absent)
//! ai-router serve [--addr 127.0.0.1:0]                      (OpenAI-compatible service)
//! Configuration: AI_ROUTER_CATALOG (catalog URL), AI_ROUTER_LOCAL (comma-separated OpenAI-compatible base URLs of
//! local servers), provider credentials in the environment variables the catalog names.

use ai_router::{server, Config, Router};
use serde_json::json;
use std::io::Read;

fn config() -> Config {
    Config {
        local: std::env::var("AI_ROUTER_LOCAL").ok().map(|v| v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()),
        ..Config::default()
    }
}

fn flag(args: &mut Vec<String>, name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.remove(i);
    (i < args.len()).then(|| args.remove(i))
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = if args.is_empty() { "help".to_owned() } else { args.remove(0) };
    let code = match cmd.as_str() {
        "models" => {
            let json_out = args.iter().any(|a| a == "--json");
            args.retain(|a| a != "--json");
            let policy = args.first().cloned().unwrap_or_else(|| "auto".to_owned());
            match Router::open(&config()) {
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
                Ok(r) => {
                    let offers = r.models(&policy);
                    if json_out {
                        println!("{}", serde_json::to_string_pretty(&offers).unwrap_or_default());
                    } else {
                        for o in &offers {
                            println!("{}\t{}\t{}\t{}", o.id(), if o.local { "local" } else if o.zero_cost() { "zero-cost" } else { "paid" }, o.cost_in + o.cost_out, o.context);
                        }
                        eprintln!("{} usable offers of {} known", offers.len(), r.offers.len());
                    }
                    0
                }
            }
        }
        "chat" => {
            let model = flag(&mut args, "--model").unwrap_or_else(|| "auto".to_owned());
            let mut prompt = args.join(" ");
            if prompt.is_empty() {
                let _ = std::io::stdin().read_to_string(&mut prompt);
            }
            match Router::open(&config()) {
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
                Ok(r) => {
                    use std::io::Write;
                    // tokens are printed as they arrive
                    let summary = r.chat_stream(&json!({ "model": model, "messages": [{ "role": "user", "content": prompt }] }), &mut |c| {
                        if let Some(t) = c["choices"][0]["delta"]["content"].as_str() {
                            print!("{t}");
                            let _ = std::io::stdout().flush();
                        }
                    });
                    println!();
                    if summary.get("error").is_some() {
                        eprintln!("{}", serde_json::to_string_pretty(&summary).unwrap_or_default());
                        1
                    } else {
                        eprintln!("[{}]", summary["router"]["offer"].as_str().unwrap_or(""));
                        0
                    }
                }
            }
        }
        "serve" => {
            let addr = flag(&mut args, "--addr").unwrap_or_else(|| "127.0.0.1:0".to_owned());
            match Router::open(&config()).and_then(|r| server::serve(r, &addr, |a| println!("ai-router http://{a}/v1"))) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
        _ => {
            println!("ai-router models [policy] [--json] | chat [--model <policy or id>] <prompt> | serve [--addr host:port]");
            i32::from(cmd != "help")
        }
    };
    std::process::exit(code);
}
