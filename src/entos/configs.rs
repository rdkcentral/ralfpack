//
// If not stated otherwise in this file or this component's LICENSE file the
// following copyright and licenses apply:
//
// Copyright 2025 Comcast Cable Communications Management, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

/// \deprecated
///
/// The platform filters for the package. These are not used on the device
/// but are used by some legacy services to determine if the package is
/// suitable for a given platform.
///
/// The configuration is an object with fields for each type of filter, for
/// example:
/// ```json
/// {
///     "platformIds": [ "SCXI11AIC", ... ],
///     "platformVariants": [ "RDK-STB", ...],
///     "propositions": [ "SKYQGW", ...],
///     "countries":  [ "NZL", ...],
///     "subdivisions": [ "GB-SCT", ...],
///     "yoctoVersions": [ "KIRKSTONE", ...]
/// }
/// ```
///
pub const ENTOS_PLATFORM_FILTERS_CONFIGURATION: &str = "urn:entos:config:platform:filters";

/// The app or service requires access to the Mediarite API (MAPI).
///
/// This configuration should contain an object like the following:
/// ```json
/// {
///     "underlay": true / false,
///     "accessGroups": {
///         <groupName>: [ <default>, <trusted>, <core> ]
///     }
/// }
/// ```
pub const ENTOS_MEDIARITE_CONFIGURATION: &str = "urn:entos:config:mediarite";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediariteConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underlay: Option<bool>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub access_groups: HashMap<String, Vec<String>>,
}

/// \deprecated
///
/// This configuration is deprecated and should not be used.  It was used
/// to denote that an app required access to the Sky Live system device.
#[deprecated]
pub const ENTOS_SKY_LIVE_CONFIGURATION: &str = "urn:entos:config:sky-live";

/// The pin management configuration for the app or service. The configuration
/// contains a string that can be one of the following values:
/// - "readwrite" - the app or service can read and write the pin
/// - "readonly" - the app or service can only read the pin
/// - "excluded" - the app or service is excluded from pin management
pub const ENTOS_PIN_MANAGEMENT_CONFIGURATION: &str = "urn:entos:config:pin-management";

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PinManagementConfiguration {
    ReadWrite,
    ReadOnly,
    Excluded,
}

impl<'de> Deserialize<'de> for PinManagementConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_ascii_lowercase().as_str() {
            "readwrite" => Ok(PinManagementConfiguration::ReadWrite),
            "readonly" => Ok(PinManagementConfiguration::ReadOnly),
            "excluded" => Ok(PinManagementConfiguration::Excluded),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["readwrite", "readonly", "excluded"],
            )),
        }
    }
}

/// The pre-launch configuration for the app. Configuration should contain
/// a string with one of the following values:
/// - "allowed" - may be pre-launched even if the app has never been launched
/// - "recent" - app may be pre-launched if it has been launched recently
/// - "never" - the app should never be pre-launched
pub const ENTOS_PRELAUNCH_CONFIGURATION: &str = "urn:entos:config:prelaunch";

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PreLaunchConfiguration {
    Allowed,
    Recent,
    Never,
}

impl<'de> Deserialize<'de> for PreLaunchConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_ascii_lowercase().as_str() {
            "allowed" => Ok(PreLaunchConfiguration::Allowed),
            "recent" => Ok(PreLaunchConfiguration::Recent),
            "never" => Ok(PreLaunchConfiguration::Never),
            _ => Err(serde::de::Error::unknown_variant(&s, &["allowed", "recent", "never"])),
        }
    }
}

/// The age rating for the app. The configuration should contain a single
/// integer value that represents the age rating.
pub const ENTOS_AGE_RATING_CONFIGURATION: &str = "urn:entos:config:age-rating";

#[derive(Debug, PartialEq, Eq)]
pub enum AgeRatingConfiguration {
    AgeRating(i32),
}

impl<'de> Deserialize<'de> for AgeRatingConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let i = i32::deserialize(deserializer)?;
        Ok(AgeRatingConfiguration::AgeRating(i))
    }
}

impl Serialize for AgeRatingConfiguration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AgeRatingConfiguration::AgeRating(i) => serializer.serialize_i32(*i),
        }
    }
}

/// The age policy configuration for the app, the configuration should contain
/// a single string describing the policy.
///
pub const ENTOS_AGE_POLICY_CONFIGURATION: &str = "urn:entos:config:age-policy";

#[derive(Debug, PartialEq, Eq)]
pub enum AgePolicyConfiguration {
    AgePolicy(String),
}

impl<'de> Deserialize<'de> for AgePolicyConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(AgePolicyConfiguration::AgePolicy(s))
    }
}

impl Serialize for AgePolicyConfiguration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AgePolicyConfiguration::AgePolicy(s) => serializer.serialize_str(s),
        }
    }
}

/// \deprecated
///
/// The content partner id for the app. The configuration should contain a
/// single string that represents the catalogue id.
pub const ENTOS_CONTENT_PARTNER_ID_CONFIGURATION: &str = "urn:entos:config:content-partner-id";

#[derive(Debug, PartialEq, Eq)]
pub enum PartnerIdConfiguration {
    PartnerId(String),
}

impl<'de> Deserialize<'de> for PartnerIdConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(PartnerIdConfiguration::PartnerId(s))
    }
}

impl Serialize for PartnerIdConfiguration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            PartnerIdConfiguration::PartnerId(s) => serializer.serialize_str(s),
        }
    }
}

/// \deprecated
///
/// The catalogue id for the app. The configuration should contain a single
/// string that represents the catalogue id.
pub const ENTOS_CATALOGUE_ID_CONFIGURATION: &str = "urn:entos:config:catalogue-id";

#[derive(Debug, PartialEq, Eq)]
pub enum CatalogueIdConfiguration {
    CatalogueId(String),
}

impl<'de> Deserialize<'de> for CatalogueIdConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(CatalogueIdConfiguration::CatalogueId(s))
    }
}

impl Serialize for CatalogueIdConfiguration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            CatalogueIdConfiguration::CatalogueId(s) => serializer.serialize_str(s),
        }
    }
}

/// Defines the list of FKPS files to be mapped into the container.
/// The configuration should contain an array of strings, each string is
/// a path to a FKPS file that should be mapped into the container.
///
/// ```json
/// {
///     "files": [
///         "file1.fkps",
///         "file2.fkps",
///         ...
///     ]
/// }
/// ```
pub const ENTOS_FKPS_CONFIGURATION: &str = "urn:entos:config:fkps";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FkpsConfiguration {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

/// \deprecated
///
/// Keep for backwards compatibility, but hasn't been used for a long time.
/// The configuration should contain a boolean value to indicate parental control
/// is enabled or not. The absence of this configuration means parental control
/// is not enabled.
pub const ENTOS_PARENTAL_CONTROL_CONFIGURATION: &str = "urn:entos:config:parental-control";

/// The app or service requires some sort of access to multicast sockets.
///
/// This configuration should contain an object like the following:
/// ```json
/// {
///     "forwarding": [
///         {
///             "address": "<multicast address>",
///             "port": <port number>
///         },
///         ...
///     ],
///     "serverSockets": [
///         {
///             "port": <port number>,
///             "address": "<multicast address>",
///             "name": <name of the socket>
///         },
///         ...
///     ],
///     "clientSockets": [
///         {
///             "name": <name of the socket>
///         },
///         ...
///     ]
/// }
/// ```
pub const ENTOS_MULTICAST_CONFIGURATION: &str = "urn:entos:config:multicast";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MulticastConfiguration {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forwarding: Vec<MulticastForwarding>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub server_sockets: Vec<MulticastServerSocket>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub client_sockets: Vec<MulticastClientSocket>,
}

#[derive(Serialize, Deserialize)]
pub struct MulticastForwarding {
    pub address: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize)]
pub struct MulticastServerSocket {
    pub port: u16,
    pub address: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct MulticastClientSocket {
    pub name: String,
}

/// \deprecated
///
/// Describes the legacy DRM configuration for the app. This configuration
/// will contain an object formatted like the following:
///
/// ```json
/// {
///     "types": [ "com.widevine.alpha", "org.w3.clearkey" ],
///     "storageSizeKB": 3000
/// }
/// ```
///
pub const ENTOS_LEGACY_DRM_CONFIGURATION: &str = "urn:entos:config:legacy:drm";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyDrmConfiguration {
    #[serde(default)]
    pub types: Vec<String>,

    #[serde(rename = "storageSizeKB", skip_serializing_if = "Option::is_none")]
    pub storage_size_kb: Option<u32>,
}

/// Icon configuration was not included in the original spec, but is used
/// to define the icon configuration for the app or service.
///
/// Note that the config follows the web manifest icon specification,
/// \see https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/icons.
///
/// This configuration should contain an object like the following:
/// ```json
/// [
///     {
///         "src": "icon/low-res.png",
///         "sizes": "48x48",
///         "type": "image/png",
///     },
///     {
///         "src": "icon/high-res.png",
///         "sizes": "192x192"
///     },
///     {
///         "src": "maskable_icon.png",
///         "sizes": "48x48",
///         "type": "image/png"
///     }
/// ]
/// ```
///
pub const ENTOS_ICONS_CONFIGURATION: &str = "urn:entos:config:icons";

pub type IconsConfiguration = Vec<IconConfiguration>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconConfiguration {
    pub src: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sizes: Option<String>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub icon_type: Option<String>,
}

/// Describes the display modes and configuration for the app.  This configuration
/// will contain an object formatted like the following:
///
/// ```json
/// {
///     "refreshRateHz": 60,
///     "pictureMode": "<name of picture mode>",
/// }
/// ```
pub const ENTOS_DISPLAY_CONFIGURATION: &str = "urn:entos:config:display";

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DisplayConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_rate_hz: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture_mode: Option<String>,
}

/// Describes the audio configuration requested for the app.  This configuration
/// will contain an object formatted like the following:
///
/// ```json
/// {
///     "soundMode": "<name of sound mode>",
///     "soundScene": "<name of sound scene>",
///     "loudnessAdjustment": <integer value in dB>
/// }
/// ```
pub const ENTOS_AUDIO_CONFIGURATION: &str = "urn:entos:config:audio";

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound_mode: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound_scene: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub loudness_adjustment: Option<i32>,
}

/// This is a rarely used config used to inform the system that the app
/// should be terminated prior to the app entering a low power state.
///
/// The config should contain a boolean value, \c true if the app should be
/// terminated prior to the device entering a low power state.
///
/// If the config is not present then it defaults to false.
pub const ENTOS_LOW_POWER_TERMINATE_CONFIGURATION: &str = "urn:entos:config:low-power:terminate";

#[derive(Debug, PartialEq, Eq)]
pub enum LowPowerTerminateConfiguration {
    LowPowerTerminate(bool),
}

impl<'de> Deserialize<'de> for LowPowerTerminateConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let b = bool::deserialize(deserializer)?;
        Ok(LowPowerTerminateConfiguration::LowPowerTerminate(b))
    }
}

impl Serialize for LowPowerTerminateConfiguration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            LowPowerTerminateConfiguration::LowPowerTerminate(b) => serializer.serialize_bool(*b),
        }
    }
}

/// Describes the marketplace intercept configuration for the app. This configuration
/// indicates whether the app launch must show intercept functionality.
///    \code{.json}
///        "configure": {
///            "urn:entos:config:marketplace-intercept": {
///                "enable": true
///            }
///        }
///    \endcode
pub const ENTOS_MARKETPLACE_INTERCEPT_CONFIGURATION: &str = "urn:entos:config:marketplace-intercept";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceInterceptConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
}
