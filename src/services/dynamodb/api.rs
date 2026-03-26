//! Async AWS DynamoDB API calls.
//!
//! All functions spawn a `tokio::task` and send the result via an
//! `UnboundedSender<Action>` so they never block the TUI event loop.

use std::collections::HashMap;

use aws_sdk_dynamodb::{
    types::{AttributeValue, KeyType},
    Client,
};
use aws_types::SdkConfig;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    services::dynamodb::{
        DdbAction,
        items::{FetchParams, ScanMode},
    },
};

// ── Key schema info ───────────────────────────────────────────────────────────

/// Partition and (optional) sort key names for a DynamoDB table.
#[derive(Debug, Clone, Default)]
pub struct TableKeyInfo {
    pub pk: String,
    pub sk: String, // empty if table has no sort key
}

// ── spawn_fetch_tables ────────────────────────────────────────────────────────

/// Spawn a task that lists all DynamoDB tables (paginated) and sends the
/// result as `Action::DynamoDB(DdbAction::TablesLoaded(...))`.
pub fn spawn_fetch_tables(cfg: SdkConfig, tx: UnboundedSender<Action>) {
    tokio::spawn(async move {
        let client = Client::new(&cfg);
        let mut tables: Vec<String> = Vec::new();
        let mut last: Option<String> = None;

        loop {
            let mut req = client.list_tables();
            if let Some(ref start) = last {
                req = req.exclusive_start_table_name(start);
            }
            match req.send().await {
                Ok(out) => {
                    tables.extend(out.table_names().iter().cloned());
                    match out.last_evaluated_table_name() {
                        Some(next) => last = Some(next.to_string()),
                        None => break,
                    }
                }
                Err(e) => {
                    let _ = tx.send(Action::AwsError(format!("ListTables: {e}")));
                    return;
                }
            }
        }
        let _ = tx.send(Action::DynamoDB(DdbAction::TablesLoaded(tables)));
    });
}

// ── spawn_fetch_items ─────────────────────────────────────────────────────────

/// Spawn a task that fetches one page of items (Scan or Query) and sends the
/// result as `Action::DynamoDB(DdbAction::ItemsLoaded {...})`.
pub fn spawn_fetch_items(params: FetchParams, tx: UnboundedSender<Action>) {
    tokio::spawn(async move {
        let client = Client::new(&params.cfg);

        // Describe table to get key schema on first call; reuse cache otherwise.
        let key_info = if params.cached_key_info.pk.is_empty() {
            match describe_table_keys(&client, &params.table_name).await {
                Ok(ki) => ki,
                Err(e) => {
                    let _ = tx.send(Action::AwsError(e));
                    return;
                }
            }
        } else {
            params.cached_key_info.clone()
        };

        let result = if params.mode == ScanMode::Query && !params.query_pk.is_empty() {
            run_query(
                &client,
                &params.table_name,
                &key_info,
                &params.query_pk,
                &params.query_sk,
                Some(params.filter_expr.as_str()).filter(|s| !s.is_empty()),
                params.start_key.clone(),
            )
            .await
        } else {
            run_scan(
                &client,
                &params.table_name,
                Some(params.filter_expr.as_str()).filter(|s| !s.is_empty()),
                params.start_key.clone(),
            )
            .await
        };

        match result {
            Ok((items, last_key)) => {
                let _ = tx.send(Action::DynamoDB(DdbAction::ItemsLoaded {
                    items,
                    last_key,
                    key_info,
                }));
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(e));
            }
        }
    });
}

// ── spawn_put_item ────────────────────────────────────────────────────────────

/// Spawn a task that marshals `json_data` → AttributeValue map and calls
/// PutItem, sending `Action::DynamoDB(DdbAction::ItemSaved)` on success.
pub fn spawn_put_item(
    cfg: SdkConfig,
    table_name: String,
    json_data: Vec<u8>,
    tx: UnboundedSender<Action>,
) {
    tokio::spawn(async move {
        // Parse JSON → serde_json::Value → AttributeValue map.
        let value: serde_json::Value = match serde_json::from_slice(&json_data) {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.send(Action::SetError(format!("invalid JSON: {e}")));
                return;
            }
        };
        let item: HashMap<String, AttributeValue> = match json_value_to_attr_map(&value) {
            Some(m) => m,
            None => {
                let _ =
                    tx.send(Action::SetError("JSON root must be an object".to_string()));
                return;
            }
        };

        let client = Client::new(&cfg);
        match client
            .put_item()
            .table_name(&table_name)
            .set_item(Some(item))
            .send()
            .await
        {
            Ok(_) => {
                let _ = tx.send(Action::DynamoDB(DdbAction::ItemSaved));
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(format!("PutItem: {e}")));
            }
        }
    });
}

// ── spawn_delete_item ─────────────────────────────────────────────────────────

/// Spawn a task that extracts the primary key from `item` and calls DeleteItem.
pub fn spawn_delete_item(
    cfg: SdkConfig,
    table_name: String,
    item: HashMap<String, AttributeValue>,
    key_info: TableKeyInfo,
    tx: UnboundedSender<Action>,
) {
    tokio::spawn(async move {
        let mut key: HashMap<String, AttributeValue> = HashMap::new();
        if let Some(pk_val) = item.get(&key_info.pk) {
            key.insert(key_info.pk.clone(), pk_val.clone());
        }
        if !key_info.sk.is_empty() {
            if let Some(sk_val) = item.get(&key_info.sk) {
                key.insert(key_info.sk.clone(), sk_val.clone());
            }
        }

        let client = Client::new(&cfg);
        match client
            .delete_item()
            .table_name(&table_name)
            .set_key(Some(key))
            .send()
            .await
        {
            Ok(_) => {
                let _ = tx.send(Action::DynamoDB(DdbAction::ItemDeleted));
            }
            Err(e) => {
                let _ = tx.send(Action::AwsError(format!("DeleteItem: {e}")));
            }
        }
    });
}

// ── Internal helpers ──────────────────────────────────────────────────────────

async fn describe_table_keys(
    client: &Client,
    table_name: &str,
) -> Result<TableKeyInfo, String> {
    let out = client
        .describe_table()
        .table_name(table_name)
        .send()
        .await
        .map_err(|e| format!("DescribeTable {table_name}: {e}"))?;

    let mut ki = TableKeyInfo::default();
    if let Some(table) = out.table() {
        for ks in table.key_schema() {
            match ks.key_type() {
                KeyType::Hash => ki.pk = ks.attribute_name().to_string(),
                KeyType::Range => ki.sk = ks.attribute_name().to_string(),
                _ => {}
            }
        }
    }
    Ok(ki)
}

async fn run_scan(
    client: &Client,
    table_name: &str,
    filter_expr: Option<&str>,
    start_key: Option<HashMap<String, AttributeValue>>,
) -> Result<
    (
        Vec<HashMap<String, AttributeValue>>,
        Option<HashMap<String, AttributeValue>>,
    ),
    String,
> {
    let mut req = client.scan().table_name(table_name).limit(50);
    if let Some(fe) = filter_expr {
        req = req.filter_expression(fe);
    }
    if let Some(sk) = start_key {
        req = req.set_exclusive_start_key(Some(sk));
    }
    let out = req
        .send()
        .await
        .map_err(|e| format!("Scan {table_name}: {e}"))?;

    let last_key = if out.last_evaluated_key().is_none_or(|m| m.is_empty()) {
        None
    } else {
        Some(out.last_evaluated_key().unwrap().to_owned())
    };
    Ok((out.items().to_vec(), last_key))
}

async fn run_query(
    client: &Client,
    table_name: &str,
    ki: &TableKeyInfo,
    pk_value: &str,
    sk_value: &str,
    filter_expr: Option<&str>,
    start_key: Option<HashMap<String, AttributeValue>>,
) -> Result<
    (
        Vec<HashMap<String, AttributeValue>>,
        Option<HashMap<String, AttributeValue>>,
    ),
    String,
> {
    let mut expr_names = HashMap::new();
    let mut expr_values: HashMap<String, AttributeValue> = HashMap::new();

    expr_names.insert("#pk".to_string(), ki.pk.clone());
    expr_values.insert(
        ":pkval".to_string(),
        AttributeValue::S(pk_value.to_string()),
    );
    let mut key_cond = "#pk = :pkval".to_string();

    if !sk_value.is_empty() && !ki.sk.is_empty() {
        expr_names.insert("#sk".to_string(), ki.sk.clone());
        expr_values.insert(
            ":skval".to_string(),
            AttributeValue::S(sk_value.to_string()),
        );
        key_cond.push_str(" AND #sk = :skval");
    }

    let mut req = client
        .query()
        .table_name(table_name)
        .key_condition_expression(key_cond)
        .set_expression_attribute_names(Some(expr_names))
        .set_expression_attribute_values(Some(expr_values))
        .limit(50);

    if let Some(fe) = filter_expr {
        req = req.filter_expression(fe);
    }
    if let Some(sk) = start_key {
        req = req.set_exclusive_start_key(Some(sk));
    }

    let out = req
        .send()
        .await
        .map_err(|e| format!("Query {table_name}: {e}"))?;

    let last_key = if out.last_evaluated_key().is_none_or(|m| m.is_empty()) {
        None
    } else {
        Some(out.last_evaluated_key().unwrap().to_owned())
    };
    Ok((out.items().to_vec(), last_key))
}

/// Convert a `serde_json::Value::Object` to a `HashMap<String, AttributeValue>`.
/// Returns `None` if the root value is not an object.
fn json_value_to_attr_map(
    value: &serde_json::Value,
) -> Option<HashMap<String, AttributeValue>> {
    let obj = value.as_object()?;
    Some(
        obj.iter()
            .map(|(k, v)| (k.clone(), json_to_attr(v)))
            .collect(),
    )
}

fn json_to_attr(v: &serde_json::Value) -> AttributeValue {
    match v {
        serde_json::Value::String(s) => AttributeValue::S(s.clone()),
        serde_json::Value::Number(n) => AttributeValue::N(n.to_string()),
        serde_json::Value::Bool(b) => AttributeValue::Bool(*b),
        serde_json::Value::Null => AttributeValue::Null(true),
        serde_json::Value::Array(arr) => {
            AttributeValue::L(arr.iter().map(json_to_attr).collect())
        }
        serde_json::Value::Object(obj) => {
            AttributeValue::M(obj.iter().map(|(k, v)| (k.clone(), json_to_attr(v))).collect())
        }
    }
}
