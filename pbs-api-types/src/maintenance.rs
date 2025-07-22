use anyhow::{bail, Error};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use proxmox_schema::{api, const_regex, ApiStringFormat, Schema, StringSchema};

const_regex! {
    pub MAINTENANCE_MESSAGE_REGEX = r"^[[:^cntrl:]]*$";
}

pub const MAINTENANCE_MESSAGE_FORMAT: ApiStringFormat =
    ApiStringFormat::Pattern(&MAINTENANCE_MESSAGE_REGEX);

pub const MAINTENANCE_MESSAGE_SCHEMA: Schema =
    StringSchema::new("Message describing the reason for the maintenance.")
        .format(&MAINTENANCE_MESSAGE_FORMAT)
        .max_length(64)
        .schema();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Operation requirements, used when checking for maintenance mode.
pub enum Operation {
    /// for any read operation like backup restore or RRD metric collection
    Read,
    /// for any write/delete operation, like backup create or GC
    Write,
    /// for any purely logical operation on the in-memory state of the datastore, e.g., to check if
    /// some mutex could be locked (e.g., GC already running?)
    ///
    /// NOTE: one must *not* do any IO operations when only helding this Op state
    Lookup,
    // GarbageCollect or Delete?
}

#[api]
#[derive(Copy, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
/// Maintenance type.
pub enum MaintenanceType {
    // TODO:
    //  - Add "GarbageCollection" or "DeleteOnly" as type and track GC (or all deletes) as separate
    //    operation, so that one can enable a mode where nothing new can be added but stuff can be
    //    cleaned
    /// Only read operations are allowed on the datastore.
    ReadOnly,
    /// Neither read nor write operations are allowed on the datastore.
    Offline,
    /// The datastore is being deleted.
    Delete,
    /// The (removable) datastore is being unmounted.
    Unmount,
    /// The S3 cache store is being refreshed.
    S3Refresh,
}
serde_plain::derive_display_from_serialize!(MaintenanceType);
serde_plain::derive_fromstr_from_deserialize!(MaintenanceType);

#[api(
    properties: {
        type: {
            type: MaintenanceType,
        },
        message: {
            optional: true,
            schema: MAINTENANCE_MESSAGE_SCHEMA,
        }
    },
    default_key: "type",
)]
#[derive(Deserialize, Serialize)]
/// Maintenance mode
pub struct MaintenanceMode {
    /// Type of maintenance ("read-only" or "offline").
    #[serde(rename = "type")]
    pub ty: MaintenanceType,

    /// Reason for maintenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl MaintenanceMode {
    /// Used for deciding whether the datastore is cleared from the internal cache
    pub fn clear_from_cache(&self) -> bool {
        self.ty == MaintenanceType::Offline
            || self.ty == MaintenanceType::Delete
            || self.ty == MaintenanceType::Unmount
    }

    pub fn check(&self, operation: Option<Operation>) -> Result<(), Error> {
        if self.ty == MaintenanceType::Delete {
            bail!("datastore is being deleted");
        }

        let message = percent_encoding::percent_decode_str(self.message.as_deref().unwrap_or(""))
            .decode_utf8()
            .unwrap_or(Cow::Borrowed(""));

        if let Some(Operation::Lookup) = operation {
            return Ok(());
        } else if self.ty == MaintenanceType::Unmount {
            bail!("datastore is being unmounted");
        } else if self.ty == MaintenanceType::Offline {
            bail!("offline maintenance mode: {}", message);
        } else if self.ty == MaintenanceType::S3Refresh {
            bail!("S3 refresh maintenance mode: {}", message);
        } else if self.ty == MaintenanceType::ReadOnly {
            if let Some(Operation::Write) = operation {
                bail!("read-only maintenance mode: {}", message);
            }
        }
        Ok(())
    }
}
