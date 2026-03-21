use proxy_daemon::HostMapping;

use crate::context::OpsContext;
use crate::helpers::{add_and_sync, list_items, remove_and_sync};
use crate::id::next_mapping_id;
use crate::params::*;
use crate::result::OpResult;

pub fn list_host_mappings(ctx: &OpsContext) -> OpResult {
    list_items(
        &ctx.store.host_mappings,
        "host mappings",
        "No host mappings configured.",
    )
}

pub async fn add_host_mapping(ctx: &OpsContext, p: AddHostMappingParams) -> OpResult {
    let id = next_mapping_id();
    let mapping = HostMapping {
        id: id.clone(),
        source_host: p.source_host,
        source_port: p.source_port,
        target_host: p.target_host,
        target_port: p.target_port,
        enabled: true,
    };

    add_and_sync(
        &ctx.store.host_mappings,
        mapping,
        &id,
        "Host mapping",
        || ctx.send_host_mappings(),
    )
    .await
}

pub async fn remove_host_mapping(ctx: &OpsContext, p: RemoveHostMappingParams) -> OpResult {
    remove_and_sync(
        &ctx.store.host_mappings,
        &p.id,
        |m| &m.id,
        "Host mapping",
        || ctx.send_host_mappings(),
    )
    .await
}
