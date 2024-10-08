//! ACME Orders data and identifiers.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::request::Request;
use crate::Error;

/// Status of an [`Order`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Status {
    /// Invalid, used as a place holder for when sending objects as contrary to account creation,
    /// the Acme RFC does not require the server to ignore unknown parts of the `Order` object.
    #[default]
    New,

    /// Authorization failed and it is now invalid.
    Invalid,

    /// The authorization is pending and the user should look through its challenges.
    ///
    /// This is the initial state of a new authorization.
    Pending,

    /// The ACME provider is processing an authorization validation.
    Processing,

    /// The requirements for the order have been met and it may be finalized.
    Ready,

    /// The certificate has been issued and can be downloaded from the URL provided in the
    /// [`Order`]'s `certificate` field.
    Valid,
}

impl Status {
    /// Serde helper
    fn is_new(&self) -> bool {
        *self == Status::New
    }

    /// Convenience method to check if the status is 'pending'.
    #[inline]
    pub fn is_pending(self) -> bool {
        self == Status::Pending
    }

    /// Convenience method to check if the status is 'valid'.
    #[inline]
    pub fn is_valid(self) -> bool {
        self == Status::Valid
    }
}

/// An identifier used for a certificate request.
///
/// Currently only supports DNS name identifiers.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum Identifier {
    /// A DNS identifier is used to request a domain name to be added to a certificate.
    Dns(String),
}

/// This contains the order data sent to and received from the ACME server.
///
/// This is typically filled with a set of domains and then issued as a new-order request via [`Account::new_order`](crate::Account::new_order).
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderData {
    /// The order status.
    #[serde(skip_serializing_if = "Status::is_new", default)]
    pub status: Status,

    /// This order's expiration date as RFC3339 formatted time string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,

    /// List of identifiers to order for the certificate.
    pub identifiers: Vec<Identifier>,

    /// An RFC3339 formatted time string. It is up to the user to choose a dev dependency for this
    /// shit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_before: Option<String>,

    /// An RFC3339 formatted time string. It is up to the user to choose a dev dependency for this
    /// shit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_after: Option<String>,

    /// Possible errors in this order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,

    /// List of URL's to authorizations the client needs to complete.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub authorizations: Vec<String>,

    /// URL the final CSR needs to be POSTed to in order to complete the order, once all
    /// authorizations have been performed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finalize: Option<String>,

    /// URL at which the issued certificate can be fetched once it is available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
}

impl OrderData {
    /// Initialize an empty order object.
    pub fn new() -> Self {
        Default::default()
    }

    /// Builder-style method to add a domain identifier to the data.
    pub fn domain(mut self, domain: String) -> Self {
        self.identifiers.push(Identifier::Dns(domain));
        self
    }
}

/// Represents an order for a new certificate. This combines the order's own location (URL) with
/// the [`OrderData`] received from the ACME server.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    /// Order location URL.
    pub location: String,

    /// The order's data object.
    pub data: OrderData,
}

impl Order {
    /// Get an authorization URL (or `None` if the index is out of range).
    pub fn authorization(&self, index: usize) -> Option<&str> {
        Some(self.data.authorizations.get(index)?)
    }

    /// Get the number of authorizations in this object.
    pub fn authorization_len(&self) -> usize {
        self.data.authorizations.len()
    }
}

/// Represents a new in-flight order creation.
///
/// This is created via [`Account::new_order`](crate::Account::new_order()).
pub struct NewOrder {
    //order: OrderData,
    /// The request to execute to place the order. When creating a [`NewOrder`] via
    /// [`Account::new_order`](crate::Account::new_order) this is guaranteed to be `Some`.
    pub request: Option<Request>,
}

impl NewOrder {
    pub(crate) fn new(request: Request) -> Self {
        Self {
            //order,
            request: Some(request),
        }
    }

    /// Deal with the response we got from the server.
    pub fn response(self, location_header: String, response_body: &[u8]) -> Result<Order, Error> {
        Ok(Order {
            location: location_header,
            data: serde_json::from_slice(response_body)
                .map_err(|err| Error::BadOrderData(err.to_string()))?,
        })
    }
}
