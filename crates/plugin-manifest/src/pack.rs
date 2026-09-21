//! Validates the `[[pack.members]]` membership section of a `kind = "pack"` marketplace
//! release manifest.
//!
//! A pack declares which plugins belong to it; it never carries the members themselves, so
//! this module only compiles the declared references into validated identities. Members are
//! spelled as bare identifiers and resolve inside the pack's own marketplace source (the
//! source-derived namespace decision); `agents` references accept a bare identifier or a full
//! canonical plugin id and are display-time filters only — they never persist anywhere.

use crate::{InvalidFieldReason, ManifestError, ManifestField, PluginName};
use ora_domain::PluginId;
use serde::Deserialize;

/// Holds the validated membership of one `kind = "pack"` release manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginPack {
    members: Vec<PackMember>,
}

impl PluginPack {
    /// Returns the declared members in manifest declaration order.
    ///
    /// Declaration order is preserved because the pack installation orchestration installs
    /// members in exactly this order; dedup and self-reference checks belong to the
    /// orchestration's preflight, not to manifest parsing.
    pub fn members(&self) -> &[PackMember] {
        &self.members
    }
}

/// One validated `[[pack.members]]` entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackMember {
    identifier: PluginName,
    agents: Vec<PackAgentRef>,
}

impl PackMember {
    /// Returns the bare member identifier.
    ///
    /// The member's full plugin id is assembled by the host as
    /// `<pack namespace>/<identifier>`, because the author cannot predict the namespace a
    /// third-party source derives from its own URL.
    pub fn identifier(&self) -> &PluginName {
        &self.identifier
    }

    /// Returns the agent-adaptation references of this member; empty means the member is
    /// agent-agnostic and belongs to every pack installation.
    pub fn agents(&self) -> &[PackAgentRef] {
        &self.agents
    }
}

/// One validated `agents` reference of a pack member.
///
/// The reference is display-time data: it is matched against installed agent plugins when a
/// pack is installed and never enters a persisted id, protocol field, or install directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackAgentRef {
    /// A bare agent identifier, matched against any installed agent plugin with the same
    /// identifier regardless of the namespace its publishing source derived.
    Bare(PluginName),
    /// A full canonical plugin id pinning the reference to one namespace and identifier.
    Canonical(PluginId),
}

/// Raw form of the `[[pack.members]]` array-of-tables under the release form's `pack` table.
#[derive(Deserialize, Clone, Debug, Eq, PartialEq)]
pub(crate) struct RawPack {
    #[serde(default)]
    members: Vec<RawPackMember>,
}

/// Raw form of one `[[pack.members]]` entry.
#[derive(Deserialize, Clone, Debug, Eq, PartialEq)]
pub(crate) struct RawPackMember {
    identifier: String,
    #[serde(default)]
    agents: Option<Vec<String>>,
}

/// Compiles the raw `pack` table into the validated membership for one manifest.
///
/// The rules keep a manifest from being half of two kinds: only `kind = "pack"` may carry the
/// table and a pack must carry it, so an accidental or missing section fails at parse time
/// instead of surfacing as an orchestrator surprise later. An empty members array parses here —
/// "no member applies to this installation" is the pack installation preflight's verdict to
/// make, and it needs the parsed-but-empty shape to do so.
pub(crate) fn compile_pack(
    kind: crate::PluginKind,
    raw: Option<RawPack>,
    field: ManifestField,
) -> Result<Option<PluginPack>, ManifestError> {
    match (kind, raw) {
        (crate::PluginKind::Pack, Some(raw)) => {
            let members = raw
                .members
                .iter()
                .enumerate()
                .map(|(index, member)| compile_member(member, index))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Some(PluginPack { members }))
        }
        (crate::PluginKind::Pack, None) => Err(invalid_field(
            field,
            InvalidFieldReason::MissingForKind { kind },
        )),
        (_, Some(_)) => Err(invalid_field(
            field,
            InvalidFieldReason::NotAllowedForKind { kind },
        )),
        (_, None) => Ok(None),
    }
}

/// Compiles one raw member into its validated identity and agent references.
fn compile_member(raw: &RawPackMember, index: usize) -> Result<PackMember, ManifestError> {
    let identifier = PluginName::parse(&raw.identifier).map_err(|reason| {
        invalid_field(ManifestField::PackMemberIdentifier { index }, reason.into())
    })?;
    let agents = raw
        .agents
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|reference| {
            compile_agent_ref(reference)
                .map_err(|reason| invalid_field(ManifestField::PackMemberAgents { index }, reason))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PackMember { identifier, agents })
}

/// Compiles one `agents` reference, accepting a bare identifier or a canonical plugin id.
///
/// A value containing `/` is the canonical spelling; anything else must be a bare identifier
/// valid under the member-identifier grammar, so a mistyped reference fails at parse time
/// rather than silently never matching an installed agent.
fn compile_agent_ref(reference: &str) -> Result<PackAgentRef, InvalidFieldReason> {
    if reference.contains('/') {
        return Ok(PackAgentRef::Canonical(PluginId::parse(reference)?));
    }
    Ok(PackAgentRef::Bare(PluginName::parse(reference)?))
}

/// Attaches a structured field path to one semantic validation reason.
fn invalid_field(field: ManifestField, reason: InvalidFieldReason) -> ManifestError {
    ManifestError::InvalidField { field, reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PluginKind, PluginManifest};
    use pretty_assertions::assert_eq;

    /// Wraps one `[[pack.members]]` body into a complete release manifest.
    fn pack_manifest(members: &str) -> String {
        format!(
            "resolver = 1\n\
             identifier = \"ora-space.python-extension-pack\"\n\
             title = \"Python Extension Pack\"\n\
             kind = \"pack\"\n\
             version = \"0.1.0\"\n\
             description = \"Python development skill, MCP, and agent tooling\"\n\
             {members}"
        )
    }

    /// Parses the manifest body and returns the compiled membership.
    fn pack_members(members: &str) -> PluginPack {
        PluginManifest::parse(&pack_manifest(members))
            .expect("pack manifest parses")
            .pack()
            .expect("pack kind carries membership")
            .clone()
    }

    /// Verifies members compile in declaration order with bare identifiers and optional agent
    /// references in both accepted spellings.
    #[test]
    fn compiles_members_in_declaration_order_with_both_agent_spellings()
    -> Result<(), Box<dyn std::error::Error>> {
        let pack = pack_members(
            "[[pack.members]]\n\
             identifier = \"ora-space.python-core\"\n\
             \n\
             [[pack.members]]\n\
             identifier = \"ora-space.claude-python-tools\"\n\
             agents = [\"ora-space.claude\", \"official/ora-space.codex\"]\n",
        );
        let members = pack.members();

        assert_eq!(members.len(), 2);
        assert_eq!(members[0].identifier().as_str(), "ora-space.python-core");
        assert!(members[0].agents().is_empty());
        assert_eq!(
            members[1].identifier().as_str(),
            "ora-space.claude-python-tools"
        );
        assert_eq!(
            members[1].agents(),
            vec![
                PackAgentRef::Bare(PluginName::parse("ora-space.claude")?),
                PackAgentRef::Canonical(PluginId::parse("official/ora-space.codex")?),
            ]
        );
        Ok(())
    }

    /// Verifies an empty members array parses: "no member applies" is the installation
    /// preflight's verdict to make, not the parser's.
    #[test]
    fn parses_an_empty_members_array() {
        assert!(
            pack_members("pack = { members = [] }\n")
                .members()
                .is_empty()
        );
    }

    /// Verifies the pack table is required exactly for `kind = "pack"`.
    #[test]
    fn requires_the_pack_table_for_pack_kind() {
        let missing = pack_manifest("");
        assert!(matches!(
            PluginManifest::parse(&missing),
            Err(ManifestError::InvalidField {
                field: ManifestField::PackMembers,
                reason: InvalidFieldReason::MissingForKind {
                    kind: PluginKind::Pack
                },
            })
        ));
    }

    /// Verifies a non-pack listing cannot carry `[[pack.members]]`.
    #[test]
    fn rejects_pack_members_on_a_non_pack_listing() {
        let source = format!(
            "resolver = 1\n\
             identifier = \"weather\"\n\
             kind = \"skill\"\n\
             version = \"1.0.0\"\n\
             description = \"Weather skill\"\n\
             [[pack.members]]\n\
             identifier = \"ora-space.python-core\"\n"
        );
        assert!(matches!(
            PluginManifest::parse(&source),
            Err(ManifestError::InvalidField {
                field: ManifestField::PackMembers,
                reason: InvalidFieldReason::NotAllowedForKind {
                    kind: PluginKind::Skill
                },
            })
        ));
    }

    /// Verifies a mistyped member identifier fails with its array position.
    #[test]
    fn rejects_a_malformed_member_identifier() {
        let source = pack_manifest("[[pack.members]]\nidentifier = \"Ora Space..bad id!\"\n");
        assert!(matches!(
            PluginManifest::parse(&source),
            Err(ManifestError::InvalidField {
                field: ManifestField::PackMemberIdentifier { index: 0 },
                ..
            })
        ));
    }

    /// Verifies a pack listing cannot declare a release of its own: it is an orchestration
    /// entry, so any `url`/`sha256` pair or `[[targets]]` array is a schema violation.
    #[test]
    fn rejects_a_release_declaration_on_a_pack_listing() {
        let with_release = pack_manifest(
            "url = \"https://example.com/pack.orax\"\nsha256 = \"abababababababababababababababababababababababababababababababab\"\n",
        );
        assert!(matches!(
            PluginManifest::parse(&with_release),
            Err(ManifestError::InvalidField {
                field: ManifestField::Url,
                reason: InvalidFieldReason::NotAllowedForKind {
                    kind: PluginKind::Pack
                },
            })
        ));
        let with_targets = pack_manifest(
            "url = \"https://example.com/pack.orax\"\nsha256 = \"abababababababababababababababababababababababababababababababab\"\n\n[[targets]]\ntarget = \"x86_64-pc-windows-msvc\"\nurl = \"https://example.com/pack-x64.orax\"\nsha256 = \"cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd\"\n",
        );
        assert!(matches!(
            PluginManifest::parse(&with_targets),
            Err(ManifestError::InvalidField {
                field: ManifestField::Url,
                reason: InvalidFieldReason::NotAllowedForKind {
                    kind: PluginKind::Pack
                },
            })
        ));
    }

    /// Verifies a pack listing carries its membership: the pack table is required, so a pack
    /// without `[[pack.members]]` is a rejected shape rather than an empty pack.
    #[test]
    fn rejects_a_pack_listing_without_the_pack_table() {
        let missing = pack_manifest("");
        assert!(matches!(
            PluginManifest::parse(&missing),
            Err(ManifestError::InvalidField {
                field: ManifestField::PackMembers,
                ..
            })
        ));
    }

    /// Verifies `kind = "pack"` cannot ship inside a package: the installed form has no
    /// orchestration surface, so the manifest parser refuses the shape outright.
    #[test]
    fn rejects_a_pack_manifest_in_the_installed_form() {
        let source = "resolver = 1\nidentifier = \"ora-space.python-extension-pack\"\nkind = \"pack\"\nversion = \"0.1.0\"\ndescription = \"Python development pack\"\n\n[[pack.members]]\nidentifier = \"ora-space.python-core\"\n";
        assert!(matches!(
            PluginManifest::parse_installed(source),
            Err(ManifestError::InvalidField {
                field: ManifestField::Kind,
                reason: InvalidFieldReason::PackNotAllowedOnInstalled,
            })
        ));
    }

    /// Verifies `marketplace_visible` is a listing attribute: an installed manifest carrying it
    /// is rejected instead of silently hiding an already-installed plugin.
    #[test]
    fn rejects_marketplace_visible_on_an_installed_manifest() {
        let source = "resolver = 1\nidentifier = \"weather\"\nkind = \"skill\"\nversion = \"1.0.0\"\ndescription = \"Weather skill\"\nmarketplace_visible = false\n";
        assert!(matches!(
            PluginManifest::parse_installed(source),
            Err(ManifestError::InvalidField {
                field: ManifestField::MarketplaceVisible,
                reason: InvalidFieldReason::MarketplaceVisibleNotAllowedOnInstalled,
            })
        ));
    }

    /// Verifies `marketplace_visible` defaults to `true` and any kind may carry it on the
    /// release form: hiding is a per-listing display decision, not a pack-only feature.
    #[test]
    fn defaults_marketplace_visible_to_true_for_every_kind()
    -> Result<(), Box<dyn std::error::Error>> {
        let manifest = PluginManifest::parse(&pack_manifest("pack = { members = [] }\n"))?;
        assert!(manifest.marketplace_visible());
        let hidden = pack_manifest("pack = { members = [] }\nmarketplace_visible = false\n");
        assert!(!PluginManifest::parse(&hidden)?.marketplace_visible());
        let skill = "resolver = 1\nidentifier = \"weather\"\nkind = \"skill\"\nversion = \"1.0.0\"\ndescription = \"Weather skill\"\nmarketplace_visible = false\n";
        assert!(!PluginManifest::parse(skill)?.marketplace_visible());
        Ok(())
    }
}
