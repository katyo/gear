use crate::Event;
use async_std::{channel::Receiver, io::Cursor};
use serde::Serialize;
use tide::{http::Url, sse, Body, Request};

#[derive(Serialize)]
struct RuleEntry {
    state: gear::RuleState,
    inputs: Vec<String>,
    outputs: Vec<String>,
}

#[derive(Serialize)]
struct RulesMap {
    goals: Vec<String>,
    rules: gear::Map<gear::RuleId, RuleEntry>,
}

#[derive(Serialize)]
struct RuleStateChangeData {
    rule: String,
    state: gear::RuleState,
}

impl From<&gear::RuleStateChange> for RuleStateChangeData {
    fn from(event: &gear::RuleStateChange) -> Self {
        Self {
            rule: event.rule.id().to_string(),
            state: event.state,
        }
    }
}

#[derive(Clone)]
pub struct Server {
    receiver: Receiver<Event>,
    scope: gear::Scope,
}

macro_rules! serve_bundled {
    ($($(#[$meta:meta])* $name:ident => $file:literal $($mime:literal)*,)*) => {
        impl Server {
            $(
                $(#[$meta])*
                #[allow(unused_mut)]
                async fn $name(_req: Request<Server>) -> tide::Result<Body> {
                    let content = include_bytes!(concat!("../web/public/", $file));
                    let mut body = Body::from_reader(Cursor::new(content), Some(content.len()));
                    $(body.set_mime($mime);)*
                    Ok(body)
                }
            )*
        }
    };
}

serve_bundled! {
    index => "index.html" "text/html;charset=utf-8",
    favicon => "favicon.png" "image/png",
    global_style => "global.css" "text/css",
    bundle_style => "bundle.css" "text/css",
    bundle_script => "bundle.js" "text/javascript",
    #[cfg(debug_assertions)]
    bundle_script_map => "bundle.js.map" "application/json",
}

impl Server {
    pub fn new(receiver: Receiver<Event>, scope: gear::Scope) -> Self {
        Self { receiver, scope }
    }

    pub fn spawn(&self, url: &Url) {
        let mut app = tide::with_state(self.clone());

        app.at("/").get(Self::index);

        app.at("/favicon.png").get(Self::favicon);
        app.at("/global.css").get(Self::global_style);
        app.at("/bundle.css").get(Self::bundle_style);
        app.at("/bundle.js").get(Self::bundle_script);
        #[cfg(debug_assertions)]
        app.at("/bundle.js.map").get(Self::bundle_script_map);

        app.at("/rules").get(Self::rules);
        app.at("/events").get(sse::endpoint(Self::events));

        let url = url.clone();
        async_std::task::spawn(async move {
            if let Err(error) = app.listen(url).await {
                log::error!("Error when starting http server: {}", error);
            }
        });
    }

    async fn rules(req: Request<Server>) -> tide::Result<Body> {
        let state = req.state();
        let store: &gear::ArtifactStore = state.scope.as_ref();

        let goals = store
            .phony
            .read()
            .iter()
            .map(|artifact| artifact.name().clone())
            .collect::<Vec<_>>();

        let rule_set = store
            .phony
            .read()
            .iter()
            .filter_map(|artifact| artifact.rule())
            .chain(
                store
                    .actual
                    .read()
                    .iter()
                    .filter_map(|artifact| artifact.rule()),
            )
            .collect::<gear::Set<_>>();

        let rules = rule_set
            .into_iter()
            .map(|rule| {
                (
                    rule.id(),
                    RuleEntry {
                        state: rule.state(),
                        inputs: rule
                            .inputs()
                            .into_iter()
                            .map(|artifact| artifact.name().clone())
                            .collect(),
                        outputs: rule
                            .outputs()
                            .into_iter()
                            .map(|artifact| artifact.name().clone())
                            .collect(),
                    },
                )
            })
            .collect();

        let output = RulesMap { goals, rules };

        Body::from_json(&output)
    }

    async fn events(req: Request<Server>, sender: sse::Sender) -> tide::Result<()> {
        let state = req.state();
        loop {
            match state.receiver.recv().await {
                Ok(Event::RulesUpdate) => sender.send("rules-update", "", None).await?,
                Ok(Event::RuleStateChange(event)) => {
                    sender
                        .send(
                            "rule-state",
                            serde_json::to_string(&RuleStateChangeData::from(&event)).unwrap(),
                            None,
                        )
                        .await?
                }
                Err(error) => {
                    log::error!("Unable to receive event due to: {}", error);
                    break;
                }
            }
        }
        Ok(())
    }
}
