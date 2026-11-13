//! Message types of the Series Events API.

use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One package of data pushed by the Series Events API: metadata plus one or
/// more [`Event`]s that are processed together, in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Transaction {
    /// Unique ID of the transaction.
    pub id: String,
    /// Internal reference for troubleshooting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// When the transaction occurred.
    pub occurred_at: jiff::Timestamp,
    /// When the transaction was published to the WebSocket.
    pub published_at: jiff::Timestamp,
    /// ID of the series.
    pub series_id: String,
    /// Order of the transaction in the series' feed. It increases but may
    /// skip values (when transactions are filtered out).
    pub sequence_number: u64,
    /// Order of the transaction within the current consumer session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_sequence_number: Option<u64>,
    /// The events, in order (at least one).
    pub events: Vec<Event>,
}

/// A single change or action within a series.
///
/// The state fields follow the Series State API model (see
/// [`series_state`](crate::series_state)) and are kept as raw JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Event {
    /// Unique ID of the event.
    pub id: String,
    /// The event type (`{actor}-{action}-{target}`), parsed; see
    /// [`EventType`] for reverted events.
    #[serde(rename = "type")]
    pub event_type: EventType,
    /// Whether the event includes the full state (depends on configuration).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub includes_full_state: Option<bool>,
    /// The entity that performed the action.
    pub actor: Entity,
    /// The action performed, for example `killed`; prefixed with `!` for
    /// reverted events.
    pub action: String,
    /// The entity the action was performed on.
    pub target: Entity,
    /// Fields that changed at the series level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series_state_delta: Option<Value>,
    /// The complete series state after the event, if included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series_state: Option<Value>,
    /// The ID of the event this event reverts; present only on reverted
    /// events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverted_event_id: Option<String>,
}

/// The actor or target of an [`Event`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Entity {
    /// The kind of entity, for example `player`, `team` or `game`.
    #[serde(rename = "type")]
    pub entity_type: String,
    /// Series-level ID of the entity.
    pub id: String,
    /// Fields that changed at the entity level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_delta: Option<Value>,
    /// The entity's state after the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<Value>,
}

macro_rules! event_types {
    ($( $(#[$doc:meta])* $variant:ident = $wire:literal, )*) => {
        /// The type of an [`Event`], `{actor}-{action}-{target}`.
        ///
        /// Each documented type has a variant; the doc of each variant names
        /// the titles GRID lists for it. `player-revived-player` appears twice
        /// in GRID's list (all titles, and Rainbow Six DBNO) and is one
        /// variant here.
        ///
        /// **Reverted events.** GRID marks an event that undoes an earlier one
        /// by prefixing its action with `!` (`player-!killed-player`). Such a
        /// type parses as [`Reverted`](Self::Reverted) wrapping the
        /// non-reverted type, so a reverted kill never matches
        /// [`PlayerKilledPlayer`](Self::PlayerKilledPlayer) by accident; use
        /// [`is_reverted`](Self::is_reverted) and [`base`](Self::base) to
        /// handle both forms. GRID's documented `player-!purchased-item`
        /// (League of Legends: player undid a purchase) is therefore
        /// `Reverted(PlayerPurchasedItem)`.
        ///
        /// Types this version of the SDK does not know become
        /// [`Unknown`](Self::Unknown). Every parsed value serializes back to
        /// the exact string it was parsed from.
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum EventType {
            $( $(#[$doc])* $variant, )*
            /// A reverted event (action prefixed with `!`); holds the type of
            /// the event being reverted.
            Reverted(Box<EventType>),
            /// A type this version of the SDK does not know, kept verbatim.
            /// In parsed values the first `-` is never followed by `!`
            /// (those are [`Reverted`](Self::Reverted)).
            Unknown(String),
        }

        impl EventType {
            fn known(value: &str) -> Option<Self> {
                match value {
                    $( $wire => Some(Self::$variant), )*
                    _ => None,
                }
            }

            fn known_str(&self) -> Option<&'static str> {
                match self {
                    $( Self::$variant => Some($wire), )*
                    Self::Reverted(_) | Self::Unknown(_) => None,
                }
            }
        }
    };
}

event_types! {
    /// Feed created (first event) (all titles).
    GridStartedFeed = "grid-started-feed",
    /// Feed ended (last event) (all titles).
    GridEndedFeed = "grid-ended-feed",
    /// Series state cannot be confirmed (all titles).
    GridInvalidatedSeries = "grid-invalidated-series",
    /// Series state is correct again (all titles).
    GridValidatedSeries = "grid-validated-series",
    /// Series begins as part of a tournament (all titles).
    TournamentStartedSeries = "tournament-started-series",
    /// Series in a tournament finished (all titles).
    TournamentEndedSeries = "tournament-ended-series",
    /// Game started in a series (all titles).
    SeriesStartedGame = "series-started-game",
    /// Game ended in a series (all titles).
    SeriesEndedGame = "series-ended-game",
    /// Team won a game (all titles).
    TeamWonGame = "team-won-game",
    /// Team won a series (all titles).
    TeamWonSeries = "team-won-series",
    /// Player killed an opponent (all titles).
    PlayerKilledPlayer = "player-killed-player",
    /// Player killed a teammate (all titles).
    PlayerTeamkilledPlayer = "player-teamkilled-player",
    /// Player killed themselves (all titles).
    PlayerSelfkilledPlayer = "player-selfkilled-player",
    /// Player rejoined after being inactive (all titles).
    PlayerRejoinedSeries = "player-rejoined-series",
    /// Player left the series (all titles).
    PlayerLeftSeries = "player-left-series",
    /// Player revived a teammate (all titles); Player revived another player from DBNO (Rainbow Six).
    PlayerRevivedPlayer = "player-revived-player",
    /// Game clock set anew (all titles).
    GameSetClock = "game-set-clock",
    /// Game clock started (all titles).
    GameStartedClock = "game-started-clock",
    /// Game clock stopped (all titles).
    GameStoppedClock = "game-stopped-clock",
    /// Timeout ended during freezetime (CS2, VALORANT).
    FreezetimeEndedTimeout = "freezetime-ended-timeout",
    /// Timeout started during freezetime (CS2, VALORANT).
    FreezetimeStartedTimeout = "freezetime-started-timeout",
    /// Round ended (CS2, VALORANT, Rainbow Six).
    GameEndedRound = "game-ended-round",
    /// A Tier NPC killed a player (League of Legends).
    GameKilledATierNpc = "game-killed-ATierNPC",
    /// Game killed a player (environmental death) (CS2, DOTA, VALORANT, Rainbow Six).
    GameKilledPlayer = "game-killed-player",
    /// S Tier NPC killed a player (League of Legends).
    GameKilledSTierNpc = "game-killed-STierNPC",
    /// A Tier NPC respawned (League of Legends).
    GameRespawnedATierNpc = "game-respawned-ATierNPC",
    /// Player respawned (DOTA, League of Legends).
    GameRespawnedPlayer = "game-respawned-player",
    /// Roshan respawned (DOTA).
    GameRespawnedRoshan = "game-respawned-roshan",
    /// S Tier NPC respawned (League of Legends).
    GameRespawnedSTierNpc = "game-respawned-STierNPC",
    /// Game clock set anew (DOTA, VALORANT).
    GameSetGameClock = "game-set-gameClock",
    /// NPC respawn clock set (DOTA, League of Legends).
    GameSetNpcRespawnClock = "game-set-npcRespawnClock",
    /// NPC respawn clock started (DOTA, League of Legends).
    GameStartedNpcRespawnClock = "game-started-npcRespawnClock",
    /// Round started (CS2, VALORANT, Rainbow Six).
    GameStartedRound = "game-started-round",
    /// NPC respawn clock stopped (DOTA, League of Legends).
    GameStoppedNpcRespawnClock = "game-stopped-npcRespawnClock",
    /// Player captured an outpost (DOTA).
    PlayerCapturedOutpost = "player-captured-outpost",
    /// Player started defusing the Spike (VALORANT).
    PlayerCompletedBeginDefuseBomb = "player-completed-beginDefuseBomb",
    /// Player started defusing bomb with kit (CS2).
    PlayerCompletedBeginDefuseWithKit = "player-completed-beginDefuseWithKit",
    /// Player started defusing bomb without kit (CS2).
    PlayerCompletedBeginDefuseWithoutKit = "player-completed-beginDefuseWithoutKit",
    /// Player captured Ultimate Orb (VALORANT).
    PlayerCompletedCaptureUltimateOrb = "player-completed-captureUltimateOrb",
    /// Player defused the Spike (VALORANT).
    PlayerCompletedDefuseBomb = "player-completed-defuseBomb",
    /// Player destroyed the Ancient (DOTA).
    PlayerCompletedDestroyAncient = "player-completed-destroyAncient",
    /// Player destroyed Melee Barracks (DOTA).
    PlayerCompletedDestroyBarracksMelee = "player-completed-destroyBarracksMelee",
    /// Player destroyed Range Barracks (DOTA).
    PlayerCompletedDestroyBarracksRange = "player-completed-destroyBarracksRange",
    /// Player completed counter-defuse (Rainbow Six).
    PlayerCompletedDestroyDefuser = "player-completed-destroyDefuser",
    /// Player destroyed a tower (DOTA, League of Legends).
    PlayerCompletedDestroyTower = "player-completed-destroyTower",
    /// Player's bomb exploded (CS2, VALORANT).
    PlayerCompletedExplodeBomb = "player-completed-explodeBomb",
    /// Player gained a level (DOTA, League of Legends).
    PlayerCompletedIncreaseLevel = "player-completed-increaseLevel",
    /// Player finished planting the bomb/Spike (CS2, VALORANT).
    PlayerCompletedPlantBomb = "player-completed-plantBomb",
    /// Player completed planting the defuser (Rainbow Six).
    PlayerCompletedPlantDefuser = "player-completed-plantDefuser",
    /// Player reached halfway point of Spike defuse (VALORANT).
    PlayerCompletedReachDefuseBombCheckpoint = "player-completed-reachDefuseBombCheckpoint",
    /// Player killed Baron Nashor (League of Legends).
    PlayerCompletedSlayBaron = "player-completed-slayBaron",
    /// Player killed Chemtech Drake (League of Legends).
    PlayerCompletedSlayChemtechDrake = "player-completed-slayChemtechDrake",
    /// Player killed Cloud Drake (League of Legends).
    PlayerCompletedSlayCloudDrake = "player-completed-slayCloudDrake",
    /// Player killed Elder Dragon (League of Legends).
    PlayerCompletedSlayElderDragon = "player-completed-slayElderDragon",
    /// Player killed Hextech Drake (League of Legends).
    PlayerCompletedSlayHextechDrake = "player-completed-slayHextechDrake",
    /// Player killed Infernal Drake (League of Legends).
    PlayerCompletedSlayInfernalDrake = "player-completed-slayInfernalDrake",
    /// Player killed Mountain Drake (League of Legends).
    PlayerCompletedSlayMountainDrake = "player-completed-slayMountainDrake",
    /// Player killed Ocean Drake (League of Legends).
    PlayerCompletedSlayOceanDrake = "player-completed-slayOceanDrake",
    /// Player killed Rift Herald (League of Legends).
    PlayerCompletedSlayRiftHerald = "player-completed-slayRiftHerald",
    /// Player killed Roshan (DOTA).
    PlayerCompletedSlayRoshan = "player-completed-slayRoshan",
    /// Player killed Void Grub (League of Legends).
    PlayerCompletedSlayVoidGrub = "player-completed-slayVoidGrub",
    /// Player stopped defusing the Spike (VALORANT).
    PlayerCompletedStopDefuseBomb = "player-completed-stopDefuseBomb",
    /// Player damaged another player (knockdown) (PUBG, Rainbow Six).
    PlayerDamagedPlayer = "player-damaged-player",
    /// Player destroyed the Ancient (DOTA).
    PlayerDestroyedAncient = "player-destroyed-ancient",
    /// Player destroyed Melee Barracks (DOTA).
    PlayerDestroyedBarracksMelee = "player-destroyed-barracksMelee",
    /// Player destroyed Range Barracks (DOTA).
    PlayerDestroyedBarracksRange = "player-destroyed-barracksRange",
    /// Player destroyed a tower (DOTA, League of Legends).
    PlayerDestroyedTower = "player-destroyed-tower",
    /// Player dropped an item (CS2, VALORANT, Rainbow Six, League of Legends).
    PlayerDroppedItem = "player-dropped-item",
    /// Player equipped an item (DOTA).
    PlayerEquippedItem = "player-equipped-item",
    /// Player lost an item (DOTA, League of Legends).
    PlayerLostItem = "player-lost-item",
    /// Player achieved a multi-kill (DOTA).
    PlayerMultikilledPlayer = "player-multikilled-player",
    /// Player picked up an item (CS2, VALORANT, Rainbow Six, League of Legends).
    PlayerPickedUpItem = "player-pickedUp-item",
    /// Player purchased an item (CS2, League of Legends).
    PlayerPurchasedItem = "player-purchased-item",
    /// Player self-damaged (DBNO) (Rainbow Six).
    PlayerSelfdamagedPlayer = "player-selfdamaged-player",
    /// Player revived themselves from DBNO (Rainbow Six).
    PlayerSelfrevivedPlayer = "player-selfrevived-player",
    /// Player sold an item (League of Legends).
    PlayerSoldItem = "player-sold-item",
    /// Player stashed an item (DOTA).
    PlayerStashedItem = "player-stashed-item",
    /// Player team-damaged another player (DBNO) (Rainbow Six).
    PlayerTeamdamagedPlayer = "player-teamdamaged-player",
    /// Player unequipped an item (DOTA).
    PlayerUnequippedItem = "player-unequipped-item",
    /// Player unstashed an item (DOTA).
    PlayerUnstashedItem = "player-unstashed-item",
    /// Player used an ability (VALORANT, DOTA, League of Legends).
    PlayerUsedAbility = "player-used-ability",
    /// Player used an item (DOTA, League of Legends).
    PlayerUsedItem = "player-used-item",
    /// End of buy/prep phase (CS2, VALORANT).
    RoundEndedFreezetime = "round-ended-freezetime",
    /// Preparation phase ended (Rainbow Six).
    RoundEndedPreparationPhase = "round-ended-preparationPhase",
    /// Start of buy/prep phase (CS2, VALORANT).
    RoundStartedFreezetime = "round-started-freezetime",
    /// Preparation phase started (Rainbow Six).
    RoundStartedPreparationPhase = "round-started-preparationPhase",
    /// Game paused (CS2, DOTA, League of Legends, Rainbow Six).
    SeriesPausedGame = "series-paused-game",
    /// Map left over and will be played (CS2, Rainbow Six).
    SeriesPickedMap = "series-picked-map",
    /// Game resumed after pause (CS2, DOTA, League of Legends, Rainbow Six).
    SeriesResumedGame = "series-resumed-game",
    /// Team banned a character/operator (DOTA, League of Legends, Rainbow Six).
    TeamBannedCharacter = "team-banned-character",
    /// Team banned a map during draft (CS2, Rainbow Six).
    TeamBannedMap = "team-banned-map",
    /// Team captured an outpost (DOTA).
    TeamCapturedOutpost = "team-captured-outpost",
    /// Team destroyed the Ancient (DOTA).
    TeamCompletedDestroyAncient = "team-completed-destroyAncient",
    /// Team destroyed Melee Barracks (DOTA).
    TeamCompletedDestroyBarracksMelee = "team-completed-destroyBarracksMelee",
    /// Team destroyed Range Barracks (DOTA).
    TeamCompletedDestroyBarracksRange = "team-completed-destroyBarracksRange",
    /// Team destroyed a tower (DOTA, League of Legends).
    TeamCompletedDestroyTower = "team-completed-destroyTower",
    /// Team killed Roshan (DOTA).
    TeamCompletedSlayRoshan = "team-completed-slayRoshan",
    /// Team destroyed the Ancient (DOTA).
    TeamDestroyedAncient = "team-destroyed-ancient",
    /// Team destroyed Melee Barracks (DOTA).
    TeamDestroyedBarracksMelee = "team-destroyed-barracksMelee",
    /// Team destroyed Range Barracks (DOTA).
    TeamDestroyedBarracksRange = "team-destroyed-barracksRange",
    /// Team destroyed a tower (DOTA, League of Legends).
    TeamDestroyedTower = "team-destroyed-tower",
    /// Team killed a player (DOTA, League of Legends).
    TeamKilledPlayer = "team-killed-player",
    /// Team killed Roshan (DOTA).
    TeamKilledRoshan = "team-killed-roshan",
    /// Team picked a character/operator (DOTA, League of Legends, Rainbow Six).
    TeamPickedCharacter = "team-picked-character",
    /// Team picked a map during draft (CS2, Rainbow Six).
    TeamPickedMap = "team-picked-map",
    /// Team won a round (CS2, VALORANT, Rainbow Six).
    TeamWonRound = "team-won-round",
}

impl EventType {
    /// Whether this is a reverted event (its action is prefixed with `!`).
    #[must_use]
    pub fn is_reverted(&self) -> bool {
        matches!(self, Self::Reverted(_))
    }

    /// The type without any revert markers: for `player-!killed-player`
    /// this is [`PlayerKilledPlayer`](Self::PlayerKilledPlayer); for a
    /// non-reverted type it is the type itself.
    #[must_use]
    pub fn base(&self) -> &Self {
        let mut ty = self;
        while let Self::Reverted(inner) = ty {
            ty = inner;
        }
        ty
    }
}

impl From<&str> for EventType {
    fn from(value: &str) -> Self {
        if let Some((actor, rest)) = value.split_once('-')
            && let Some(rest) = rest.strip_prefix('!')
        {
            return Self::Reverted(Box::new(Self::from(format!("{actor}-{rest}").as_str())));
        }
        Self::known(value).unwrap_or_else(|| Self::Unknown(value.to_owned()))
    }
}

impl From<String> for EventType {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl FromStr for EventType {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(value))
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reverted(inner) => {
                let inner = inner.to_string();
                match inner.split_once('-') {
                    Some((actor, rest)) => write!(f, "{actor}-!{rest}"),
                    None => write!(f, "!{inner}"),
                }
            }
            Self::Unknown(value) => f.write_str(value),
            known => f.write_str(known.known_str().unwrap_or_default()),
        }
    }
}

impl Serialize for EventType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for EventType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = std::borrow::Cow::<'de, str>::deserialize(deserializer)?;
        Ok(Self::from(&*value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_unknown_and_reverted_types() {
        assert_eq!(
            EventType::from("player-killed-player"),
            EventType::PlayerKilledPlayer
        );
        assert_eq!(
            EventType::from("game-killed-ATierNPC"),
            EventType::GameKilledATierNpc
        );
        assert_eq!(
            EventType::from("player-!killed-player"),
            EventType::Reverted(Box::new(EventType::PlayerKilledPlayer))
        );
        assert_eq!(
            EventType::from("player-!purchased-item"),
            EventType::Reverted(Box::new(EventType::PlayerPurchasedItem))
        );
        assert_eq!(
            EventType::from("drone-!zapped-player"),
            EventType::Reverted(Box::new(EventType::Unknown("drone-zapped-player".into())))
        );
        assert_eq!(
            EventType::from("drone-zapped-player"),
            EventType::Unknown("drone-zapped-player".into())
        );
        let reverted = EventType::from("player-!killed-player");
        assert!(reverted.is_reverted());
        assert_eq!(reverted.base(), &EventType::PlayerKilledPlayer);
        assert!(!EventType::TeamWonRound.is_reverted());
        assert_eq!(EventType::TeamWonRound.base(), &EventType::TeamWonRound);
    }

    #[test]
    fn round_trips_to_the_same_string() {
        for wire in [
            "grid-started-feed",
            "player-pickedUp-item",
            "game-respawned-STierNPC",
            "player-!killed-player",
            "player-!!killed-player",
            "player-!purchased-item",
            "drone-!zapped-player",
            "something",
            "-!x",
            "a-!",
            "",
        ] {
            let parsed = EventType::from(wire);
            assert_eq!(parsed.to_string(), wire);
            let json = serde_json::to_value(&parsed).unwrap();
            assert_eq!(json, Value::String(wire.into()));
            let back: EventType = serde_json::from_value(json).unwrap();
            assert_eq!(back, parsed);
        }
    }
}
