export type Skin = "sun" | "tide" | "storm" | "grove";
export type Element = "ember" | "frost" | "storm" | "thorn";
export type Form = "bolt" | "orbit" | "swarm" | "nova";
export type Phase = "lobby" | "running" | "victory" | "defeat";
export type EnemyKind = "mireling" | "midge" | "spitter" | "brute" | "colossus";
export type CharmKind = "damage" | "speed" | "magnet";
export interface Vec2 { x: number; y: number }
export interface PlayerView { id:string;name:string;skin:Skin;pos:Vec2;hp:number;max_hp:number;level:number;xp:number;xp_next:number;downed:boolean;connected:boolean;element:Element;form:Form;kills:number;damage:number;revives:number }
export interface EnemyView { id:number;kind:EnemyKind;pos:Vec2;hp:number;max_hp:number;boss:boolean }
export interface ProjectileView { id:number;pos:Vec2;element:Element;hostile:boolean;size:number }
export interface PickupView { id:number;pos:Vec2;kind:string;owner_id:string|null }
export interface TelegraphView { id:number;pos:Vec2;radius:number;remaining_ms:number }
export interface UpgradeChoice { id:number;kind:string;title:string;description:string }
export interface UpgradeDraft { id:number;choices:UpgradeChoice[];expires_ms:number }
export interface RuneDraft { id:number;incantation:string;element:Element;form:Form;spell_name:string;description:string;confidence:number }
export interface BogMetrics { fold_commits:number;materialized_players:number;materialized_enemies:number;hnsw_queries:number;ese_compiles:number;enemy_counts:[EnemyKind,number][];charms:[CharmKind,number][];top_damage:[string,number][];party_hp_milli:number;party_max_hp_milli:number;downed_players:number;total_damage:number }
export interface DirectorView { mode:string;pressure:number;spawn_interval_ms:number;reason:string }
export interface GameSnapshot { tick:number;phase:Phase;run_id:number;elapsed_ms:number;remaining_ms:number;wave:number;wave_label:string;join_url:string;connected_players:number;you:string|null;players:PlayerView[];enemies:EnemyView[];projectiles:ProjectileView[];pickups:PickupView[];telegraphs:TelegraphView[];boss_hp:[number,number]|null;upgrade_draft:UpgradeDraft|null;rune_draft:RuneDraft|null;announcement:string|null;metrics:BogMetrics;director:DirectorView }
export interface Profile { player_id:string;name:string;skin:Skin }
export interface InputVector { x:number;y:number }
