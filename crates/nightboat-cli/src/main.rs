use std::path::PathBuf;
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use fx_core::content::ContentFormat;
use fx_core::models::CreateArticle;
use serde::{Deserialize, Serialize};

const CONFIG_DIR: &str = "nightboat";
const CONFIG_FILE: &str = "cli.json";

#[derive(Parser)]
#[command(name = "nbt", about = "NightBoat CLI — upload and manage articles")]
struct Cli {
    /// Server URL (default: http://localhost:3847)
    #[arg(long, env = "NBT_SERVER", default_value = "http://localhost:3847")]
    server: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Login via AT Protocol OAuth (opens browser), or with --password for platform-local users
    Login {
        /// Handle (e.g. user.bsky.social or dzming.li)
        handle: Option<String>,
        /// Platform-local password (skips OAuth, uses /auth/login)
        #[arg(long)]
        password: Option<String>,
    },
    /// Show current logged-in user
    Me,
    /// List recent articles
    #[command(alias = "ls")]
    List {
        /// Max number of articles to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// List available tags
    Tags,
    /// Upload a local file as a new article
    Upload {
        /// Path to .md, .typ, or .html file
        file: PathBuf,
        /// Article title (defaults to filename)
        #[arg(short, long)]
        title: Option<String>,
        /// Short description
        #[arg(short, long)]
        desc: Option<String>,
        /// Language code (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// Tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Prereqs (tag_id:type, e.g. calculus:required,linalg:recommended)
        #[arg(long, value_delimiter = ',')]
        prereqs: Vec<String>,
        /// License (default: CC-BY-SA-4.0)
        #[arg(long, default_value = "CC-BY-SA-4.0")]
        license: String,
        /// Category (e.g. general, lecture, paper, review, or custom)
        #[arg(long, default_value = "general")]
        category: String,
        /// Book ID (for reviews)
        #[arg(long)]
        book_id: Option<String>,
        /// Series ID — add this article to a series
        #[arg(long)]
        series: Option<String>,
        /// Resource files to upload to the series repo (e.g. references.bib)
        #[arg(long, value_delimiter = ',')]
        resource: Vec<PathBuf>,
        // -- Paper metadata (only for --category paper) --
        /// Venue (e.g. CVPR, NeurIPS, Nature)
        #[arg(long)]
        venue: Option<String>,
        /// Venue type (conference, journal, preprint, workshop, thesis)
        #[arg(long)]
        venue_type: Option<String>,
        /// Publication year
        #[arg(long)]
        year: Option<i16>,
        /// DOI
        #[arg(long)]
        doi: Option<String>,
        /// arXiv ID (e.g. 2406.12345)
        #[arg(long)]
        arxiv_id: Option<String>,
        /// Paper has been accepted
        #[arg(long)]
        accepted: bool,
        // -- Experience metadata (only for --category experience) --
        /// Experience kind (postgrad, interview, competition, application, other)
        #[arg(long)]
        exp_kind: Option<String>,
        /// Target school/company/competition
        #[arg(long)]
        target: Option<String>,
        /// Result (accepted, rejected, pending, passed, failed)
        #[arg(long)]
        result: Option<String>,
    },
    /// Update an existing article's content from a local file
    Update {
        /// Article AT URI
        uri: String,
        /// Path to .md, .typ, or .html file (updates content if provided)
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// New title
        #[arg(short, long)]
        title: Option<String>,
        /// New description
        #[arg(short, long)]
        desc: Option<String>,
    },
    /// Delete an article
    Delete {
        /// Article AT URI
        uri: String,
    },
    /// Get article content (source + rendered HTML)
    Get {
        /// Article AT URI
        uri: String,
        /// Output source to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Ask a question
    Question {
        /// Question title
        title: String,
        /// Language code (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// Tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Handles to invite to answer (comma-separated, e.g. alice,bob)
        #[arg(long, value_delimiter = ',')]
        invite: Vec<String>,
    },
    /// Manage skill trees
    Tree {
        #[command(subcommand)]
        action: TreeCommand,
    },
    /// Manage books (create, update, add editions)
    Book {
        #[command(subcommand)]
        action: BookCommand,
    },
    /// Manage courses and sessions
    Course {
        #[command(subcommand)]
        action: CourseCommand,
    },
    /// Admin operations (manage platform users, publish as any user)
    Admin {
        #[command(subcommand)]
        action: AdminCommand,
    },
    /// Logout (remove saved token)
    Logout,
}

#[derive(Subcommand)]
enum AdminCommand {
    /// Create a platform user
    #[command(name = "create-user")]
    CreateUser {
        /// User handle
        handle: String,
        /// Password
        password: String,
        /// Display name
        #[arg(long)]
        display_name: Option<String>,
    },
    /// List all platform users
    #[command(name = "list-users", alias = "users")]
    ListUsers,
    /// Set a localized name for a tag
    #[command(name = "set-tag-name")]
    SetTagName {
        /// Tag ID
        id: String,
        /// Locale code (e.g. zh, en, fr)
        locale: String,
        /// Localized name
        name: String,
    },
    /// Add an alias for a tag (e.g. "CV" -> "computer-vision")
    #[command(name = "add-tag-alias")]
    AddTagAlias {
        /// Tag ID
        id: String,
        /// Alias
        alias: String,
    },
    /// Remove a tag alias
    #[command(name = "rm-tag-alias")]
    RmTagAlias {
        /// Alias to remove
        alias: String,
    },
    /// Merge one tag into another (migrate all references)
    #[command(name = "merge-tag")]
    MergeTag {
        /// Source tag ID (will be deleted)
        #[arg(long)]
        from: String,
        /// Target tag ID (will absorb references)
        #[arg(long)]
        into: String,
    },
    /// Create a series as a platform user
    #[command(name = "create-series")]
    CreateSeries {
        /// Platform user handle
        #[arg(long)]
        r#as: String,
        /// Series title
        #[arg(short, long)]
        title: String,
        /// Short description
        #[arg(short, long)]
        desc: Option<String>,
        /// Topic tags (comma-separated, e.g. cs,math)
        #[arg(long)]
        topics: Option<String>,
        /// Parent series ID (for sub-series)
        #[arg(long)]
        parent: Option<String>,
        /// Language code (default: zh)
        #[arg(short, long)]
        lang: Option<String>,
        /// Source series ID this is a translation of
        #[arg(long)]
        translation_of: Option<String>,
    },
    /// Upload a cover image for a series (admin override)
    #[command(name = "upload-series-cover")]
    UploadSeriesCover {
        /// Series ID (e.g. s-223mjladc6xr7)
        #[arg(long)]
        id: String,
        /// Path to image file (jpg, png, webp; max 5 MB)
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Reference an existing file in the series' pijul repo as its cover
    #[command(name = "set-series-cover-ref")]
    SetSeriesCoverRef {
        /// Series ID (e.g. s-223mjladc6xr7)
        #[arg(long)]
        id: String,
        /// Relative path inside the repo (e.g. figures/logo.png)
        #[arg(short, long)]
        file: String,
    },
    /// Add an article to a series
    #[command(name = "add-to-series")]
    AddToSeries {
        /// Series ID
        #[arg(long)]
        series: String,
        /// Article AT URI
        #[arg(long)]
        article: String,
    },
    /// Update an article's content (admin override, no auth needed)
    Update {
        /// Article AT URI
        #[arg(long)]
        uri: String,
        /// Path to .md, .typ, or .html file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// New title
        #[arg(short, long)]
        title: Option<String>,
        /// New description
        #[arg(short, long)]
        desc: Option<String>,
    },
    /// Ban a user (by DID or handle)
    #[command(name = "ban-user")]
    BanUser {
        /// DID or handle of the user to ban
        did_or_handle: String,
        /// Reason for the ban
        #[arg(long)]
        reason: Option<String>,
    },
    /// Unban a user (by DID or handle)
    #[command(name = "unban-user")]
    UnbanUser {
        /// DID or handle of the user to unban
        did_or_handle: String,
    },
    /// List all banned users
    #[command(name = "banned-users")]
    BannedUsers,
    /// Delete an article (admin override, soft-delete with 30-day appeal window)
    #[command(name = "delete-article")]
    DeleteArticle {
        /// Article AT URI
        uri: String,
        /// Reason for deletion
        #[arg(long)]
        reason: Option<String>,
    },
    /// Set article visibility (public, cn_hidden, unlisted, pending_review, removed)
    #[command(name = "set-visibility")]
    SetVisibility {
        /// Article AT URI
        uri: String,
        /// Visibility: public, cn_hidden, unlisted, pending_review, removed
        visibility: String,
        /// Reason (shown to author for cn_hidden/removed)
        #[arg(long)]
        reason: Option<String>,
    },
    /// List pending appeals
    #[command(name = "appeals")]
    Appeals,
    /// Resolve an appeal (approve or reject)
    #[command(name = "resolve-appeal")]
    ResolveAppeal {
        /// Appeal ID
        id: String,
        /// "approved" or "rejected"
        #[arg(long)]
        status: String,
        /// Admin response message
        #[arg(long)]
        response: Option<String>,
    },
    /// Merge two questions (move answers from one to another)
    #[command(name = "merge-questions")]
    MergeQuestions {
        /// Source question URI (will be deleted)
        #[arg(long)]
        from: String,
        /// Target question URI (will absorb answers)
        #[arg(long)]
        into: String,
    },
    /// Publish a question as a platform user
    #[command(name = "publish-question")]
    PublishQuestion {
        /// Platform user handle to publish as
        #[arg(long)]
        r#as: String,
        /// Path to .md, .typ, or .html file
        #[arg(short, long)]
        file: PathBuf,
        /// Question title
        #[arg(short, long)]
        title: Option<String>,
        /// Short description
        #[arg(short, long)]
        desc: Option<String>,
        /// Language code (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// Tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },
    /// Post an answer to a question as a platform user
    #[command(name = "publish-answer")]
    PublishAnswer {
        /// Platform user handle to publish as
        #[arg(long)]
        r#as: String,
        /// Question AT URI to answer
        #[arg(long)]
        question: String,
        /// Path to .md, .typ, or .html file
        #[arg(short, long)]
        file: PathBuf,
        /// Answer title
        #[arg(short, long)]
        title: Option<String>,
        /// Short description
        #[arg(short, long)]
        desc: Option<String>,
        /// Language code (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
    },
    /// Verify a user's credentials (education + affiliation)
    #[command(name = "verify-credentials")]
    VerifyCredentials {
        /// DID or handle
        did_or_handle: String,
        /// Education entries as JSON: [{"degree":"PhD","school":"MIT","year":"2024","current":false}]
        #[arg(long)]
        education: Option<String>,
        /// Current affiliation
        #[arg(long)]
        affiliation: Option<String>,
    },
    /// Revoke a user's credentials verification
    #[command(name = "revoke-credentials")]
    RevokeCredentials {
        /// DID or handle
        did_or_handle: String,
    },
    /// Revert a book edit by edit log ID
    #[command(name = "revert-book-edit")]
    RevertBookEdit {
        /// Edit log ID to revert
        edit_id: String,
    },
    /// Show edit history for a book
    #[command(name = "book-history")]
    BookHistory {
        /// Book ID
        book_id: String,
    },
    /// Publish an article as a platform user
    Publish {
        /// Platform user handle to publish as
        #[arg(long)]
        r#as: String,
        /// Path to .md, .typ, or .html file
        #[arg(short, long)]
        file: PathBuf,
        /// Article title (defaults to filename)
        #[arg(short, long)]
        title: Option<String>,
        /// Short description
        #[arg(short, long)]
        desc: Option<String>,
        /// Category (e.g. general, lecture, paper, review, or custom)
        #[arg(long, default_value = "general")]
        category: String,
        /// Book ID (for reviews)
        #[arg(long)]
        book_id: Option<String>,
        /// Language code (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// Tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// License (default: CC-BY-SA-3.0)
        #[arg(long, default_value = "CC-BY-SA-3.0")]
        license: String,
        /// AT URI of the article this is a translation of
        #[arg(long)]
        translation_of: Option<String>,
        /// Series ID — add this article to a series
        #[arg(long)]
        series: Option<String>,
        /// Resource files to upload to the series repo (e.g. references.bib)
        #[arg(long, value_delimiter = ',')]
        resource: Vec<PathBuf>,
    },
    /// Import a directory tree as series chapters with resources
    ///
    /// Each subdirectory containing a markdown/typst file becomes a chapter.
    /// All other files (images, etc.) are uploaded as series resources.
    ///
    /// Example: fx admin import-dir --as ustclug --series s-xxx --dir ./linux101/
    ///
    /// Expected layout:
    ///   ch01-intro/index.md
    ///   ch01-intro/images/logo.png
    ///   ch02-config/index.md
    ///   ch02-config/images/screenshot.jpg
    #[command(name = "import-dir")]
    ImportDir {
        /// Platform user handle to publish as
        #[arg(long)]
        r#as: String,
        /// Series ID to import into (must already exist)
        #[arg(long)]
        series: String,
        /// Root directory to import
        #[arg(short, long)]
        dir: PathBuf,
        /// Language code
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// License
        #[arg(long, default_value = "CC-BY-SA-4.0")]
        license: String,
        /// Tags for all chapters (comma-separated)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Dry run: show what would be imported without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Import a repository directory as a series using batch-publish (single pijul change)
    ///
    /// Reads a TOML manifest listing articles and their repo paths, plus
    /// auto-collects all image files. Everything is uploaded in one batch request.
    ///
    /// Manifest format:
    /// ```toml
    /// [[article]]
    /// title = "Chapter 1"
    /// path = "ch1/intro.md"
    ///
    /// [[article]]
    /// title = "Chapter 2"
    /// path = "ch2/main.md"
    /// ```
    ///
    /// Image files (.png/.jpg/.gif/.svg) referenced in the markdown are auto-detected
    /// and included. Or specify `image_dirs` to include all images from directories.
    #[command(name = "import-repo")]
    ImportRepo {
        /// Platform user handle to publish as
        #[arg(long)]
        r#as: String,
        /// Series ID
        #[arg(long)]
        series: String,
        /// Root directory of the repo
        #[arg(short, long)]
        dir: PathBuf,
        /// TOML manifest file listing articles (relative to dir)
        #[arg(short, long)]
        manifest: PathBuf,
        /// Language code
        #[arg(short, long, default_value = "en")]
        lang: String,
        /// License
        #[arg(long, default_value = "CC-BY-SA-4.0")]
        license: String,
        /// Tags for all articles (comma-separated)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Extra directories to scan for images (relative to dir, comma-separated)
        #[arg(long, value_delimiter = ',')]
        image_dirs: Vec<PathBuf>,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum BookCommand {
    /// List all books
    #[command(alias = "ls")]
    List,
    /// Create a new book (with its first edition)
    Create {
        /// Book title
        #[arg(short, long)]
        title: String,
        /// Subtitle
        #[arg(short = 'S', long)]
        subtitle: Option<String>,
        /// Authors (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        authors: Vec<String>,
        /// Description
        #[arg(short, long)]
        desc: Option<String>,
        /// Cover image URL
        #[arg(long)]
        cover_url: Option<String>,
        /// Tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Prereq tags (comma-separated tag IDs)
        #[arg(long, value_delimiter = ',')]
        prereqs: Vec<String>,
        // -- First edition fields --
        /// Edition title (e.g. "Fourth Edition")
        #[arg(long, default_value = "First Edition")]
        edition: String,
        /// Language code (e.g. zh, en, ja)
        #[arg(short, long, default_value = "en")]
        lang: String,
        /// ISBN
        #[arg(long)]
        isbn: Option<String>,
        /// Publisher
        #[arg(long)]
        publisher: Option<String>,
        /// Year
        #[arg(long)]
        year: Option<String>,
        /// Translators (comma-separated)
        #[arg(long, value_delimiter = ',')]
        translators: Vec<String>,
        /// Purchase links as JSON: [{"label":"Amazon","url":"https://..."}]
        #[arg(long)]
        purchase_links: Option<String>,
        /// Cover image URL for this edition
        #[arg(long)]
        edition_cover_url: Option<String>,
        /// Subtitle of the first edition
        #[arg(long)]
        edition_subtitle: Option<String>,
    },
    /// Update a book's info
    Update {
        /// Book ID (e.g. bk-xxx)
        id: String,
        /// New title
        #[arg(short, long)]
        title: Option<String>,
        /// New description
        #[arg(short, long)]
        desc: Option<String>,
        /// New cover URL
        #[arg(long)]
        cover_url: Option<String>,
        /// Edit summary
        #[arg(long)]
        summary: Option<String>,
    },
    /// Add an edition to a book
    #[command(name = "add-edition")]
    AddEdition {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Edition title (localized title of this edition, e.g. Chinese translation title)
        #[arg(short, long)]
        title: String,
        /// Edition subtitle
        #[arg(short = 'S', long)]
        subtitle: Option<String>,
        /// Edition name (e.g. "Fourth Edition", "Revised Edition"). Defaults to title.
        #[arg(short = 'n', long)]
        edition_name: Option<String>,
        /// Language code (e.g. zh, en, ja)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// ISBN
        #[arg(long)]
        isbn: Option<String>,
        /// Publisher
        #[arg(long)]
        publisher: Option<String>,
        /// Year
        #[arg(long)]
        year: Option<String>,
        /// Translators (comma-separated)
        #[arg(long, value_delimiter = ',')]
        translators: Vec<String>,
        /// Purchase links as JSON: [{"label":"Amazon","url":"https://..."}]
        #[arg(long)]
        purchase_links: Option<String>,
        /// Cover image URL for this edition
        #[arg(long)]
        cover_url: Option<String>,
    },
    /// Update an existing edition's info (only supplied fields change)
    #[command(name = "update-edition")]
    UpdateEdition {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Edition ID
        #[arg(long)]
        edition_id: String,
        /// Edition title (localized title of this edition)
        #[arg(short, long)]
        title: Option<String>,
        /// Edition subtitle (pass empty string to clear)
        #[arg(short = 'S', long)]
        subtitle: Option<String>,
        /// Edition name (e.g. "Fourth Edition")
        #[arg(short = 'n', long)]
        edition_name: Option<String>,
        /// Language code (e.g. zh, en, ja)
        #[arg(short, long)]
        lang: Option<String>,
        /// ISBN
        #[arg(long)]
        isbn: Option<String>,
        /// Publisher
        #[arg(long)]
        publisher: Option<String>,
        /// Year
        #[arg(long)]
        year: Option<String>,
        /// Translators (comma-separated, replaces existing)
        #[arg(long, value_delimiter = ',')]
        translators: Option<Vec<String>>,
        /// Purchase links as JSON (replaces existing)
        #[arg(long)]
        purchase_links: Option<String>,
        /// Cover image URL for this edition
        #[arg(long)]
        cover_url: Option<String>,
    },
    /// Show a book's detail
    Show {
        /// Book ID
        id: String,
    },
    /// Upload a cover image for a book edition
    #[command(name = "upload-cover")]
    UploadCover {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Edition ID
        #[arg(long)]
        edition_id: String,
        /// Path to image file
        file: PathBuf,
    },
    /// Add a chapter to a book's table of contents
    #[command(name = "add-chapter")]
    AddChapter {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Chapter title
        #[arg(short, long)]
        title: String,
        /// Parent chapter ID (for sub-chapters)
        #[arg(long)]
        parent_id: Option<String>,
        /// Order index (0-based)
        #[arg(long, default_value = "0")]
        order: i32,
        /// Linked article URI
        #[arg(long)]
        article_uri: Option<String>,
        /// Tags this chapter teaches (comma-separated)
        #[arg(long, value_delimiter = ',')]
        teaches: Vec<String>,
        /// Prereq tags as "tag_id:required" or "tag_id:recommended" (comma-separated)
        #[arg(long, value_delimiter = ',')]
        prereqs: Vec<String>,
    },
    /// Upload a directory of chapters from a TOML manifest
    ///
    /// The TOML file describes chapters (with optional file paths to upload as articles).
    /// Example manifest: see `fx book upload-chapters --help`.
    #[command(name = "upload-chapters")]
    UploadChapters {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Path to chapters TOML manifest
        manifest: PathBuf,
        /// Language for uploaded articles (default: zh)
        #[arg(short, long, default_value = "zh")]
        lang: String,
        /// License for uploaded articles (default: CC-BY-SA-4.0)
        #[arg(long, default_value = "CC-BY-SA-4.0")]
        license: String,
        /// Dry run — print what would happen without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Add a supplementary resource to a book (solutions, videos, slides, etc.)
    #[command(name = "add-resource")]
    AddResource {
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Resource kind (solutions, exercises, video, slides, errata, code, other)
        #[arg(short, long)]
        kind: String,
        /// Display label
        #[arg(short, long)]
        label: String,
        /// URL
        #[arg(short, long)]
        url: String,
        /// Edition ID (omit for all editions)
        #[arg(long)]
        edition_id: Option<String>,
        /// Display order
        #[arg(long, default_value = "0")]
        position: i16,
    },
}

#[derive(Subcommand)]
enum CourseCommand {
    /// List published courses
    #[command(alias = "ls")]
    List,
    /// Show course detail
    Show {
        /// Course ID (e.g. crs-xxx)
        id: String,
    },
    /// Create a new course
    Create {
        /// Course title
        #[arg(short, long)]
        title: String,
        /// Course code (e.g. 18.404)
        #[arg(long)]
        code: Option<String>,
        /// Description
        #[arg(short, long)]
        desc: Option<String>,
        /// Institution
        #[arg(long)]
        institution: Option<String>,
        /// Department
        #[arg(long)]
        department: Option<String>,
        /// Semester (e.g. Fall 2020)
        #[arg(long)]
        semester: Option<String>,
        /// Language (default: en)
        #[arg(short, long, default_value = "en")]
        lang: String,
        /// Source URL (e.g. OCW link)
        #[arg(long)]
        source_url: Option<String>,
        /// Source attribution
        #[arg(long)]
        source_attribution: Option<String>,
    },
    /// Update course metadata
    Update {
        /// Course ID
        id: String,
        /// New title
        #[arg(short, long)]
        title: Option<String>,
        /// New description
        #[arg(short, long)]
        desc: Option<String>,
        /// Publish/unpublish
        #[arg(long)]
        publish: Option<bool>,
    },
    /// Add a session (lecture) to a course
    #[command(name = "add-session")]
    AddSession {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Topic
        #[arg(short, long)]
        topic: Option<String>,
        /// Date
        #[arg(long)]
        date: Option<String>,
        /// Readings (e.g. "Sipser Ch. 1.1-1.3")
        #[arg(short, long)]
        readings: Option<String>,
        /// Resources as "type:label:url" (e.g. "video:Lecture:https://...", "notes:Handout:https://...")
        #[arg(long, value_delimiter = ',')]
        resource: Vec<String>,
        /// Sort order (auto-increments if omitted)
        #[arg(long)]
        order: Option<i32>,
        /// Tags (comma-separated tag IDs) — what this session covers
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Prereq tags (comma-separated tag IDs) — what you should know
        #[arg(long, value_delimiter = ',')]
        prereqs: Vec<String>,
    },
    /// Update a session
    #[command(name = "update-session")]
    UpdateSession {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Session ID
        #[arg(long)]
        session_id: String,
        /// Topic
        #[arg(short, long)]
        topic: Option<String>,
        /// Readings
        #[arg(short, long)]
        readings: Option<String>,
        /// Resources as "type:label:url" (replaces all resources)
        #[arg(long, value_delimiter = ',')]
        resource: Vec<String>,
    },
    /// Delete a session
    #[command(name = "rm-session")]
    RmSession {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Session ID
        #[arg(long)]
        session_id: String,
    },
    /// Add a tag to the course
    #[command(name = "add-tag")]
    AddTag {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Tag ID
        tag_id: String,
    },
    /// Remove a tag from the course
    #[command(name = "rm-tag")]
    RmTag {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Tag ID
        tag_id: String,
    },
    /// Add a textbook to the course
    #[command(name = "add-textbook")]
    AddTextbook {
        /// Course ID
        #[arg(long)]
        course_id: String,
        /// Book ID
        #[arg(long)]
        book_id: String,
        /// Role (required/recommended/supplementary)
        #[arg(long, default_value = "required")]
        role: String,
    },
    /// Import sessions from a TOML file
    Import {
        /// Course ID
        course_id: String,
        /// Path to TOML file
        file: PathBuf,
    },
}

#[derive(Subcommand)]
enum TreeCommand {
    /// List all community skill trees
    #[command(alias = "ls")]
    List,
    /// Show a skill tree's edges
    Show {
        /// Skill tree AT URI
        uri: String,
    },
    /// Create a skill tree from a TOML file
    Create {
        /// Path to a TOML file defining the tree
        file: PathBuf,
    },
    /// Export a skill tree to a TOML file for editing
    Export {
        /// Skill tree AT URI
        uri: String,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import/sync edges from a TOML file into an existing tree
    Import {
        /// Skill tree AT URI
        uri: String,
        /// Path to TOML file with edges
        file: PathBuf,
    },
    /// Add a single edge
    #[command(name = "add-edge")]
    AddEdge {
        /// Skill tree AT URI
        uri: String,
        /// Parent tag ID
        parent: String,
        /// Child tag ID
        child: String,
    },
    /// Remove a single edge
    #[command(name = "rm-edge")]
    RmEdge {
        /// Skill tree AT URI
        uri: String,
        /// Parent tag ID
        parent: String,
        /// Child tag ID
        child: String,
    },
    /// Fork a skill tree
    Fork {
        /// Source skill tree AT URI
        uri: String,
    },
    /// Adopt a skill tree as your active tree
    Adopt {
        /// Skill tree AT URI
        uri: String,
    },
    /// Add a prerequisite relationship between two tags
    #[command(name = "add-prereq")]
    AddPrereq {
        /// Source tag (must be mastered first)
        from: String,
        /// Target tag (requires the source)
        to: String,
        /// Prereq type: required or recommended
        #[arg(long, default_value = "required")]
        prereq_type: String,
    },
    /// Remove a prerequisite relationship
    #[command(name = "rm-prereq")]
    RmPrereq {
        /// Source tag
        from: String,
        /// Target tag
        to: String,
    },
    /// List all prerequisite relationships
    #[command(name = "list-prereqs", alias = "prereqs")]
    ListPrereqs,
}

/// TOML manifest for uploading book chapters.
///
/// Example:
/// ```toml
/// [[chapter]]
/// key = "ch1"
/// title = "第一章：绪论"
/// file = "ch01.typ"
/// order = 0
/// teaches = ["cat-intro"]
///
/// [[chapter]]
/// key = "ch1-1"
/// parent = "ch1"
/// title = "1.1 基本定义"
/// file = "ch01-01.typ"
/// order = 0
/// teaches = ["cat-def"]
/// prereqs = [{ tag = "cat-intro", type = "required" }]
/// ```
#[derive(Serialize, Deserialize)]
struct ChapterManifest {
    #[serde(default, rename = "chapter")]
    chapters: Vec<ChapterEntry>,
}

#[derive(Serialize, Deserialize)]
struct ChapterEntry {
    /// Locally unique key for referencing as a parent
    #[serde(default)]
    key: String,
    /// Title shown in the table of contents
    title: String,
    /// Path to a file to upload as an article (relative to the manifest file)
    #[serde(default)]
    file: Option<PathBuf>,
    /// Key of the parent chapter (omit for top-level)
    #[serde(default)]
    parent: Option<String>,
    /// Display order among siblings (0-based)
    #[serde(default)]
    order: i32,
    /// Tag IDs this chapter teaches
    #[serde(default)]
    teaches: Vec<String>,
    /// Prereq tags
    #[serde(default)]
    prereqs: Vec<ChapterPrereqEntry>,
}

#[derive(Serialize, Deserialize)]
struct ChapterPrereqEntry {
    tag: String,
    #[serde(default = "default_prereq_type")]
    r#type: String,
}
fn default_prereq_type() -> String { "required".to_string() }

/// TOML format for skill tree files
#[derive(Serialize, Deserialize)]
struct TreeFile {
    title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    uri: Option<String>,
    edges: Vec<TreeEdge>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
struct TreeEdge {
    parent: String,
    child: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    server: Option<String>,
    token: Option<String>,
    did: Option<String>,
    handle: Option<String>,
    admin_secret: Option<String>,
}

impl Config {
    fn path() -> Result<PathBuf> {
        let dir = dirs_next::config_dir()
            .context("Cannot determine config directory")?
            .join(CONFIG_DIR);
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join(CONFIG_FILE))
    }

    fn load() -> Result<Self> {
        let path = Self::path()?;
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            Ok(Self::default())
        }
    }

    fn save(&self) -> Result<()> {
        let path = Self::path()?;
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn token(&self) -> Result<&str> {
        self.token.as_deref().context("Not logged in. Run: nbt login <handle>")
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

struct OAuthCallbackResult {
    token: String,
    did: String,
    handle: String,
}

/// Accept a single HTTP request on the local listener, extract token from query params,
/// respond with a success page, then return the result.
async fn accept_oauth_callback(listener: tokio::net::TcpListener) -> Result<OAuthCallbackResult> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut stream, _) = listener.accept().await?;
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the GET request line to extract query params
    let path = request.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("");

    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");

    let params: std::collections::HashMap<&str, String> = query.split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            Some((k, urlencoding::decode(v).unwrap_or_default().into_owned()))
        })
        .collect();

    let token = params.get("token").cloned()
        .context("No token in callback")?;
    let did = params.get("did").cloned().unwrap_or_default();
    let handle = params.get("handle").cloned().unwrap_or_default();

    // Send a simple success response
    let body = "<!DOCTYPE html><html><body><h2>Login successful!</h2><p>You can close this tab and return to the terminal.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;

    Ok(OAuthCallbackResult { token, did, handle })
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    // Use server from CLI flag, falling back to saved config
    let server = if cli.server != "http://localhost:3847" {
        cli.server.clone()
    } else {
        config.server.clone().unwrap_or(cli.server.clone())
    };
    let base = format!("{}/api", server.trim_end_matches('/'));

    match cli.command {
        Command::Login { handle, password } => {
            if let Some(password) = password {
                // Password-based login (platform-local users)
                let handle = handle.context("Handle is required with --password")?;

                let resp: serde_json::Value = client()
                    .post(format!("{base}/auth/login"))
                    .json(&serde_json::json!({ "identifier": handle, "password": password }))
                    .send().await?
                    .error_for_status().context("Login failed")?
                    .json().await?;

                config.server = Some(server);
                config.token = resp["token"].as_str().map(String::from);
                config.did = resp["did"].as_str().map(String::from);
                config.handle = resp["handle"].as_str().map(String::from);
                config.save()?;

                let display = resp["handle"].as_str().unwrap_or("?");
                let did = resp["did"].as_str().unwrap_or("?");
                println!("Logged in as {display} ({did})");
            } else {
                // OAuth login (default): open browser, listen for callback
                let handle = handle.context("Handle is required. Usage: nbt login <handle>")?;
                let server_base = server.trim_end_matches('/');

                // Bind to a random port
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await
                    .context("Failed to bind local server")?;
                let port = listener.local_addr()?.port();
                let callback_url = format!("http://localhost:{port}/callback");

                let login_url = format!(
                    "{server_base}/oauth/login?handle={}&cli_redirect={}",
                    urlencoding::encode(&handle),
                    urlencoding::encode(&callback_url),
                );

                println!("Opening browser for AT Protocol authorization...");
                if open::that(&login_url).is_err() {
                    println!("Open this URL in your browser:\n  {login_url}");
                }

                // Wait for the callback (with timeout)
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(120),
                    accept_oauth_callback(listener),
                ).await
                    .context("Login timed out (2 minutes). Try again.")?
                    .context("OAuth callback failed")?;

                config.server = Some(server);
                config.token = Some(result.token);
                config.did = Some(result.did.clone());
                config.handle = Some(result.handle.clone());
                config.save()?;

                println!("Logged in as {} ({})", result.handle, result.did);
            }
        }

        Command::Me => {
            let token = config.token()?;
            let resp: serde_json::Value = client()
                .get(format!("{base}/auth/me"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("Not authenticated")?
                .json().await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }

        Command::List { limit } => {
            let articles: Vec<serde_json::Value> = client()
                .get(format!("{base}/articles"))
                .send().await?
                .error_for_status()?
                .json().await?;

            for a in articles.iter().take(limit) {
                let uri = a["at_uri"].as_str().unwrap_or("");
                let title = a["title"].as_str().unwrap_or("(untitled)");
                let author = a["author_handle"].as_str().unwrap_or("?");
                let format = a["content_format"].as_str().unwrap_or("?");
                let votes = a["vote_score"].as_i64().unwrap_or(0);
                println!("{title}  [{format}] by {author}  votes:{votes}");
                println!("  {uri}");
            }
            if articles.is_empty() {
                println!("No articles found.");
            }
        }

        Command::Tags => {
            let tags: Vec<serde_json::Value> = client()
                .get(format!("{base}/tags"))
                .send().await?
                .error_for_status()?
                .json().await?;

            for tag in &tags {
                let id = tag["id"].as_str().unwrap_or("");
                let name = tag["name"].as_str().unwrap_or("");
                println!("{id}\t{name}");
            }
            if tags.is_empty() {
                println!("No tags found.");
            }
        }

        Command::Question { title, lang, tags, invite } => {
            let token = config.token()?;
            let body = CreateArticle {
                title: title.clone(),
                summary: None,
                content: String::new(),
                content_format: fx_core::content::ContentFormat::Markdown,
                lang: Some(lang),
                license: Some("CC-BY-SA-4.0".to_string()),
                translation_of: None,
                restricted: None,
                category: None,
                tags,
                prereqs: vec![],
                series_id: None,
                metadata: None,
                authors: vec![],
                invites: invite,
            };
            let article: serde_json::Value = client()
                .post(format!("{base}/questions"))
                .bearer_auth(&token)
                .json(&body)
                .send().await?
                .error_for_status().context("Failed to post question")?
                .json().await?;
            let uri = article["at_uri"].as_str().unwrap_or("?");
            println!("Asked: {title}");
            println!("URI:   {uri}");
            if !body.invites.is_empty() {
                println!("Invited: {}", body.invites.join(", "));
            }
        }

        Command::Upload { file, title, desc, lang, tags, prereqs, license, category, book_id, series, resource,
                          venue, venue_type, year, doi, arxiv_id, accepted,
                          exp_kind, target, result } => {
            let token = config.token()?;

            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;

            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            let (content_format, content) = match ext {
                "md" | "markdown" => (ContentFormat::Markdown, content),
                "typ" | "typst" => (ContentFormat::Typst, content),
                "html" | "htm" => (ContentFormat::Html, content),
                _ => bail!("Unsupported file extension: .{ext} (use .md, .typ, or .html)"),
            };

            if content_format == ContentFormat::Html {
                validate_html_fragment(&content)?;
            }

            let title = title.unwrap_or_else(|| {
                file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

            let parsed_prereqs = parse_prereqs(&prereqs)?;

            let cat_metadata = if venue.is_some() || doi.is_some() || arxiv_id.is_some() || (year.is_some() && category == "paper") || accepted {
                Some(fx_core::models::CategoryMetadata::Paper(fx_core::models::CreatePaperMetadata {
                    venue, venue_type, year, doi, arxiv_id, accepted,
                }))
            } else if exp_kind.is_some() || target.is_some() || result.is_some() {
                Some(fx_core::models::CategoryMetadata::Experience(fx_core::models::CreateExperienceMetadata {
                    kind: exp_kind, target, year, result,
                }))
            } else if book_id.is_some() {
                Some(fx_core::models::CategoryMetadata::Review {
                    book_id, edition_id: None, course_id: None,
                })
            } else {
                None
            };

            let body = CreateArticle {
                title: title.clone(),
                summary: desc,
                content,
                content_format,
                lang: Some(lang),
                license: Some(license),
                translation_of: None,
                restricted: None,
                category: Some(category),
                tags,
                prereqs: parsed_prereqs,
                series_id: series.clone(),
                metadata: cat_metadata,
                authors: vec![],
                invites: vec![],
            };

            // Collect resource files (expanding directories recursively)
            let mut resource_files: Vec<(PathBuf, String)> = Vec::new();
            for res_path in &resource {
                if res_path.is_dir() {
                    for entry in walkdir::WalkDir::new(res_path).into_iter().filter_map(|e| e.ok()) {
                        if entry.file_type().is_file() {
                            let path = entry.path().to_path_buf();
                            let rel = path.strip_prefix(res_path.parent().unwrap_or(res_path))
                                .unwrap_or(&path)
                                .to_string_lossy().into_owned();
                            resource_files.push((path, rel));
                        }
                    }
                } else {
                    let name = res_path.file_name()
                        .and_then(|n| n.to_str())
                        .context("Invalid resource filename")?
                        .to_string();
                    resource_files.push((res_path.clone(), name));
                }
            }

            let resp: serde_json::Value = if resource_files.is_empty() {
                // Simple JSON upload (no resources)
                client()
                    .post(format!("{base}/articles"))
                    .bearer_auth(token)
                    .json(&body)
                    .send().await?
                    .error_for_status().context("Upload failed")?
                    .json().await?
            } else {
                // Multipart upload: metadata + resources in one request
                let metadata = serde_json::to_string(&body)?;
                let mut form = reqwest::multipart::Form::new()
                    .text("metadata", metadata);

                for (abs_path, rel_name) in &resource_files {
                    let file_bytes = std::fs::read(abs_path)
                        .with_context(|| format!("Cannot read {}", abs_path.display()))?;
                    let part = reqwest::multipart::Part::bytes(file_bytes)
                        .file_name(rel_name.clone());
                    form = form.part("resources", part);
                    println!("  + {rel_name}");
                }

                let resp = client()
                    .post(format!("{base}/articles/upload"))
                    .bearer_auth(token)
                    .multipart(form)
                    .send().await?;
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    bail!("Upload failed ({status}): {body}");
                }
                resp
                    .json().await?
            };

            let uri = resp["at_uri"].as_str().unwrap_or("?");
            println!("Published: {title}");
            println!("URI: {uri}");

            // Add to series if specified
            if let Some(ref series_id) = series {
                client()
                    .post(format!("{base}/series/{series_id}/articles"))
                    .bearer_auth(token)
                    .json(&serde_json::json!({ "article_uri": uri }))
                    .send().await?
                    .error_for_status().context("Failed to add article to series")?;
                println!("Added to series: {series_id}");
            }
        }

        Command::Update { uri, file, title, desc } => {
            let token = config.token()?;

            let content = if let Some(ref path) = file {
                Some(std::fs::read_to_string(path)
                    .with_context(|| format!("Cannot read {}", path.display()))?)
            } else {
                None
            };

            let body = serde_json::json!({
                "uri": uri,
                "title": title,
                "description": desc,
                "content": content,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/articles/update"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update failed")?
                .json().await?;

            let title = resp["title"].as_str().unwrap_or("?");
            println!("Updated: {title}");
        }

        Command::Delete { uri } => {
            let token = config.token()?;

            client()
                .post(format!("{base}/articles/delete"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "uri": uri }))
                .send().await?
                .error_for_status().context("Delete failed")?;

            println!("Deleted: {uri}");
        }

        Command::Get { uri, output } => {
            let article: serde_json::Value = client()
                .get(format!("{base}/articles/by-uri"))
                .query(&[("uri", &uri)])
                .send().await?
                .error_for_status().context("Article not found")?
                .json().await?;

            let content: serde_json::Value = client()
                .get(format!("{base}/articles/by-uri/content"))
                .query(&[("uri", &uri)])
                .send().await?
                .error_for_status()?
                .json().await?;

            let title = article["title"].as_str().unwrap_or("?");
            let source = content["source"].as_str().unwrap_or("");

            if let Some(path) = output {
                std::fs::write(&path, source)
                    .with_context(|| format!("Cannot write {}", path.display()))?;
                println!("{title} -> {}", path.display());
            } else {
                println!("# {title}\n");
                println!("{source}");
            }
        }

        Command::Tree { action } => {
            handle_tree(&base, &config, action).await?;
        }

        Command::Course { action } => {
            handle_course(&base, &config, action).await?;
        }

        Command::Book { action } => {
            handle_book(&base, &config, action).await?;
        }

        Command::Admin { action } => {
            handle_admin(&base, &mut config, action).await?;
        }

        Command::Logout => {
            if let Ok(token) = config.token() {
                let _ = client()
                    .post(format!("{base}/auth/logout"))
                    .bearer_auth(token)
                    .send().await;
            }
            config.token = None;
            config.did = None;
            config.handle = None;
            config.save()?;
            println!("Logged out.");
        }
    }

    Ok(())
}

async fn handle_book(base: &str, config: &Config, action: BookCommand) -> Result<()> {
    let token = config.token()?;
    match action {
        BookCommand::List => {
            let resp: Vec<serde_json::Value> = client()
                .get(format!("{base}/books"))
                .send().await?
                .error_for_status().context("List books failed")?
                .json().await?;

            if resp.is_empty() {
                println!("No books yet.");
            } else {
                for b in &resp {
                    let id = b["id"].as_str().unwrap_or("?");
                    let title = b["title"].as_str().unwrap_or("?");
                    let authors = b["authors"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                        .unwrap_or_default();
                    println!("  {id}  {title}  ({authors})");
                }
                println!("{} book(s)", resp.len());
            }
        }

        BookCommand::Create { title, subtitle, authors, desc, cover_url, tags, prereqs,
                             edition, lang, isbn, publisher, year, translators, purchase_links, edition_cover_url, edition_subtitle } => {
            // Title/subtitle/desc can be JSON like {"en":"...","zh":"..."} or plain string
            let parse_i18n = |s: &str| -> serde_json::Value {
                if s.starts_with('{') {
                    serde_json::from_str(s).unwrap_or(serde_json::json!({"en": s}))
                } else {
                    serde_json::json!({"en": s})
                }
            };
            let title_val = parse_i18n(&title);
            let subtitle_val = subtitle.as_deref().map(parse_i18n).unwrap_or(serde_json::json!({}));
            let desc_val = desc.as_deref().map(parse_i18n).unwrap_or(serde_json::json!({}));
            let body = serde_json::json!({
                "title": title_val,
                "subtitle": subtitle_val,
                "authors": authors,
                "description": desc_val,
                "cover_url": cover_url,
                "tags": tags,
                "prereqs": prereqs,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/books"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Create book failed")?
                .json().await?;

            let book_id = resp["id"].as_str().unwrap_or("?");
            if let Some(warning) = resp["warning"].as_str() {
                eprintln!("Warning: {warning}");
            }
            println!("Created book: {title}");
            println!("ID: {book_id}");

            // Auto-create the first edition
            let links: Vec<serde_json::Value> = if let Some(ref pl) = purchase_links {
                serde_json::from_str(pl).context("Invalid JSON for --purchase-links")?
            } else {
                vec![]
            };
            let ed_body = serde_json::json!({
                "book_id": book_id,
                "title": title,
                "subtitle": edition_subtitle,
                "edition_name": edition,
                "lang": lang,
                "isbn": isbn,
                "publisher": publisher,
                "year": year,
                "translators": translators,
                "purchase_links": links,
                "cover_url": edition_cover_url,
            });

            let ed_response = client()
                .post(format!("{base}/books/{book_id}/editions"))
                .bearer_auth(token)
                .json(&ed_body)
                .send().await?;
            if !ed_response.status().is_success() {
                let status = ed_response.status();
                let body = ed_response.text().await.unwrap_or_default();
                bail!("Create first edition failed ({status}): {body}");
            }
            let ed_resp: serde_json::Value = ed_response.json().await?;

            let eid = ed_resp["id"].as_str().unwrap_or("?");
            println!("Created edition: {edition} ({lang})");
            println!("Edition ID: {eid}");
        }

        BookCommand::Update { id, title, desc, cover_url, summary } => {
            let body = serde_json::json!({
                "id": id,
                "title": title,
                "description": desc,
                "cover_url": cover_url,
                "edit_summary": summary,
            });

            client()
                .post(format!("{base}/books/update"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update book failed")?;

            println!("Updated book {id}");
        }

        BookCommand::AddEdition { book_id, title, subtitle, edition_name, lang, isbn, publisher, year, translators, purchase_links, cover_url } => {
            let links: Vec<serde_json::Value> = if let Some(ref pl) = purchase_links {
                serde_json::from_str(pl).context("Invalid JSON for --purchase-links")?
            } else {
                vec![]
            };

            let edition_name = edition_name.unwrap_or_else(|| title.clone());
            let body = serde_json::json!({
                "book_id": book_id,
                "title": title,
                "subtitle": subtitle,
                "edition_name": edition_name,
                "lang": lang,
                "isbn": isbn,
                "publisher": publisher,
                "year": year,
                "translators": translators,
                "purchase_links": links,
                "cover_url": cover_url,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/books/{book_id}/editions"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Add edition failed")?
                .json().await?;

            let eid = resp["id"].as_str().unwrap_or("?");
            println!("Added edition to book {book_id}: {title} ({lang})");
            println!("Edition ID: {eid}");
        }

        BookCommand::UpdateEdition {
            book_id, edition_id, title, subtitle, edition_name, lang,
            isbn, publisher, year, translators, purchase_links, cover_url,
        } => {
            let current: serde_json::Value = client()
                .get(format!("{base}/books/{book_id}"))
                .send().await?
                .error_for_status().context("Get book failed")?
                .json().await?;
            let ed = current["editions"].as_array()
                .and_then(|eds| eds.iter().find(|e| e["id"].as_str() == Some(&edition_id)))
                .with_context(|| format!("Edition {edition_id} not found on book {book_id}"))?;

            let merge_str = |new: Option<String>, key: &str| -> Option<String> {
                new.or_else(|| ed[key].as_str().map(String::from))
            };
            let title = merge_str(title, "title").context("title missing")?;
            let edition_name = merge_str(edition_name, "edition_name").unwrap_or_else(|| title.clone());
            let lang = merge_str(lang, "lang").context("lang missing")?;
            let subtitle = subtitle
                .map(|s| if s.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(s) })
                .unwrap_or_else(|| ed["subtitle"].clone());
            let isbn = merge_str(isbn, "isbn").map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
            let publisher = merge_str(publisher, "publisher").map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
            let year = merge_str(year, "year").map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
            let cover_url = merge_str(cover_url, "cover_url").map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
            let translators = match translators {
                Some(v) => serde_json::Value::Array(v.into_iter().map(serde_json::Value::String).collect()),
                None => ed["translators"].clone(),
            };
            let purchase_links = match purchase_links {
                Some(pl) => serde_json::from_str(&pl).context("Invalid JSON for --purchase-links")?,
                None => ed["purchase_links"].clone(),
            };

            let body = serde_json::json!({
                "title": title,
                "subtitle": subtitle,
                "edition_name": edition_name,
                "lang": lang,
                "isbn": isbn,
                "publisher": publisher,
                "year": year,
                "translators": translators,
                "purchase_links": purchase_links,
                "cover_url": cover_url,
            });

            client()
                .put(format!("{base}/books/{book_id}/editions/{edition_id}"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update edition failed")?;

            println!("Updated edition {edition_id}");
        }

        BookCommand::UploadCover { book_id, edition_id, file } => {
            let file_bytes = std::fs::read(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let file_name = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("cover.jpg");
            let part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string());
            let form = reqwest::multipart::Form::new().part("file", part);

            client()
                .post(format!("{base}/books/{book_id}/editions/{edition_id}/cover"))
                .bearer_auth(token)
                .multipart(form)
                .send().await?
                .error_for_status().context("Upload cover failed")?;

            println!("Uploaded cover for edition {edition_id}");
        }

        BookCommand::Show { id } => {
            let resp: serde_json::Value = client()
                .get(format!("{base}/books/{id}"))
                .send().await?
                .error_for_status().context("Get book failed")?
                .json().await?;

            let book = &resp["book"];
            println!("Title: {}", book["title"].as_str().unwrap_or("?"));
            println!("Authors: {}", book["authors"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_default());
            if let Some(d) = book["description"].as_str() {
                if !d.is_empty() { println!("Description: {d}"); }
            }

            if let Some(editions) = resp["editions"].as_array() {
                if !editions.is_empty() {
                    println!("\nEditions:");
                    for ed in editions {
                        let etitle = ed["title"].as_str().unwrap_or("?");
                        let elang = ed["lang"].as_str().unwrap_or("?");
                        let eisbn = ed["isbn"].as_str().unwrap_or("-");
                        println!("  [{elang}] {etitle}  ISBN: {eisbn}");
                    }
                }
            }

            let review_count = resp["review_count"].as_u64().unwrap_or(0);
            println!("\n{review_count} review(s)");

            // Show chapters
            if let Some(chapters) = resp["chapters"].as_array() {
                if !chapters.is_empty() {
                    println!("\nTable of Contents:");
                    for ch in chapters {
                        let ctitle = ch["title"].as_str().unwrap_or("?");
                        let cid = ch["id"].as_str().unwrap_or("?");
                        let indent = if ch["parent_id"].is_null() { "" } else { "  " };
                        println!("  {indent}{ctitle}  ({cid})");
                    }
                }
            }
        }

        BookCommand::AddChapter { book_id, title, parent_id, order, article_uri, teaches, prereqs } => {
            let token = config.token()?;
            let prereqs_json: Vec<serde_json::Value> = prereqs.iter().map(|p| {
                let (tag_id, prereq_type) = p.split_once(':').unwrap_or((p, "required"));
                serde_json::json!({ "tag_id": tag_id, "prereq_type": prereq_type })
            }).collect();
            let body = serde_json::json!({
                "title": title,
                "parent_id": parent_id,
                "order_index": order,
                "article_uri": article_uri,
                "teaches": teaches,
                "prereqs": prereqs_json,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/books/{book_id}/chapters"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Add chapter failed")?
                .json().await?;

            let cid = resp["id"].as_str().unwrap_or("?");
            println!("Added chapter: {title} ({cid})");
        }

        BookCommand::UploadChapters { book_id, manifest, lang, license, dry_run } => {
            let token = config.token()?;
            let manifest_dir = manifest.parent().unwrap_or(std::path::Path::new("."));
            let manifest_text = std::fs::read_to_string(&manifest)
                .with_context(|| format!("Cannot read {}", manifest.display()))?;
            let cm: ChapterManifest = toml::from_str(&manifest_text)
                .context("Invalid chapters TOML")?;

            // key → created chapter ID
            let mut key_to_id: std::collections::HashMap<String, String> = std::collections::HashMap::new();

            for (i, ch) in cm.chapters.iter().enumerate() {
                let parent_id = ch.parent.as_ref().and_then(|k| key_to_id.get(k)).cloned();

                // Upload article if file is specified
                let article_uri: Option<String> = if let Some(ref rel_path) = ch.file {
                    let abs_path = manifest_dir.join(rel_path);
                    let content = std::fs::read_to_string(&abs_path)
                        .with_context(|| format!("Cannot read {}", abs_path.display()))?;
                    let ext = abs_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    let fmt = match ext { "typ" => "typst", "html" => "html", _ => "markdown" };

                    if dry_run {
                        println!("[dry-run] Would upload {} as article ({fmt})", abs_path.display());
                        None
                    } else {
                        let article_body = serde_json::json!({
                            "title": ch.title,
                            "content": content,
                            "content_format": fmt,
                            "lang": lang,
                            "license": license,
                            "tags": [],
                            "prereqs": [],
                            "book_id": book_id,
                        });
                        let article_resp: serde_json::Value = client()
                            .post(format!("{base}/articles"))
                            .bearer_auth(token)
                            .json(&article_body)
                            .send().await
                            .with_context(|| format!("Upload failed for chapter {}", i + 1))?
                            .error_for_status()
                            .with_context(|| format!("Server rejected chapter {} article", i + 1))?
                            .json().await?;
                        let uri = article_resp["at_uri"].as_str()
                            .map(String::from)
                            .context("No at_uri in article response")?;
                        println!("  Uploaded article: {uri}");
                        Some(uri)
                    }
                } else {
                    None
                };

                let prereqs_json: Vec<serde_json::Value> = ch.prereqs.iter().map(|p| {
                    serde_json::json!({ "tag_id": p.tag, "prereq_type": p.r#type })
                }).collect();

                let chapter_body = serde_json::json!({
                    "title": ch.title,
                    "parent_id": parent_id,
                    "order_index": ch.order,
                    "article_uri": article_uri,
                    "teaches": ch.teaches,
                    "prereqs": prereqs_json,
                });

                if dry_run {
                    println!("[dry-run] Would create chapter: {} (order={}, parent={:?}, teaches={:?})",
                        ch.title, ch.order, ch.parent, ch.teaches);
                    if !ch.key.is_empty() {
                        key_to_id.insert(ch.key.clone(), format!("dry-{}", i));
                    }
                    continue;
                }

                let chapter_resp: serde_json::Value = client()
                    .post(format!("{base}/books/{book_id}/chapters"))
                    .bearer_auth(token)
                    .json(&chapter_body)
                    .send().await
                    .with_context(|| format!("Create chapter failed: {}", ch.title))?
                    .error_for_status()
                    .with_context(|| format!("Server rejected chapter: {}", ch.title))?
                    .json().await?;

                let cid = chapter_resp["id"].as_str().unwrap_or("?").to_string();
                println!("Chapter: {} ({cid})", ch.title);
                if !ch.key.is_empty() {
                    key_to_id.insert(ch.key.clone(), cid);
                }
            }

            if !dry_run {
                println!("\nDone. {} chapter(s) created.", cm.chapters.len());
            }
        }

        BookCommand::AddResource { book_id, kind, label, url, edition_id, position } => {
            let token = config.token()?;
            let body = serde_json::json!({
                "kind": kind,
                "label": label,
                "url": url,
                "edition_id": edition_id,
                "position": position,
            });
            let resp: serde_json::Value = client()
                .post(format!("{base}/books/{book_id}/resources"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Add resource failed")?
                .json().await?;
            let id = resp["id"].as_str().unwrap_or("?");
            println!("Added resource: {label} ({id})");
        }
    }
    Ok(())
}

async fn handle_tree(base: &str, config: &Config, action: TreeCommand) -> Result<()> {
    match action {
        TreeCommand::List => {
            let trees: Vec<serde_json::Value> = client()
                .get(format!("{base}/skill-trees"))
                .send().await?
                .error_for_status()?
                .json().await?;

            if trees.is_empty() {
                println!("No skill trees found.");
                return Ok(());
            }

            for t in &trees {
                let uri = t["at_uri"].as_str().unwrap_or("");
                let title = t["title"].as_str().unwrap_or("(untitled)");
                let field = t["field"].as_str().unwrap_or("-");
                let edges = t["edge_count"].as_i64().unwrap_or(0);
                let adopts = t["adopt_count"].as_i64().unwrap_or(0);
                let author = t["author_handle"].as_str()
                    .or_else(|| t["did"].as_str())
                    .unwrap_or("?");
                println!("{title}  [{field}]  {edges} edges  {adopts} adopts  by {author}");
                println!("  {uri}");
            }
        }

        TreeCommand::Show { uri } => {
            let detail: serde_json::Value = client()
                .get(format!("{base}/skill-trees/by-uri"))
                .query(&[("uri", &uri)])
                .send().await?
                .error_for_status().context("Skill tree not found")?
                .json().await?;

            let title = detail["tree"]["title"].as_str().unwrap_or("?");
            let field = detail["tree"]["field"].as_str().unwrap_or("-");
            let desc = detail["tree"]["description"].as_str().unwrap_or("");
            println!("{title}  [{field}]");
            if !desc.is_empty() {
                println!("{desc}");
            }
            println!();

            if let Some(edges) = detail["edges"].as_array() {
                for e in edges {
                    let p = e["parent_tag"].as_str().unwrap_or("?");
                    let c = e["child_tag"].as_str().unwrap_or("?");
                    println!("  {p} -> {c}");
                }
                println!("\n{} edges total", edges.len());
            }
        }

        TreeCommand::Create { file } => {
            let token = config.token()?;
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let tree_file: TreeFile = toml::from_str(&content)
                .with_context(|| format!("Invalid TOML in {}", file.display()))?;

            let edges: Vec<serde_json::Value> = tree_file.edges.iter().map(|e| {
                serde_json::json!({ "parent_tag": e.parent, "child_tag": e.child })
            }).collect();

            let body = serde_json::json!({
                "title": tree_file.title,
                "description": tree_file.description,
                "field": tree_file.field,
                "edges": edges,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/skill-trees"))
                .bearer_auth(token)
                .json(&body)
                .send().await?
                .error_for_status().context("Create skill tree failed")?
                .json().await?;

            let uri = resp["at_uri"].as_str().unwrap_or("?");
            println!("Created: {}", tree_file.title);
            println!("URI: {uri}");
        }

        TreeCommand::Export { uri, output } => {
            let detail: serde_json::Value = client()
                .get(format!("{base}/skill-trees/by-uri"))
                .query(&[("uri", &uri)])
                .send().await?
                .error_for_status().context("Skill tree not found")?
                .json().await?;

            let tree_file = TreeFile {
                title: detail["tree"]["title"].as_str().unwrap_or("").to_string(),
                description: detail["tree"]["description"].as_str().map(String::from),
                field: detail["tree"]["field"].as_str().map(String::from),
                uri: Some(uri),
                edges: detail["edges"].as_array()
                    .map(|arr| arr.iter().map(|e| TreeEdge {
                        parent: e["parent_tag"].as_str().unwrap_or("").to_string(),
                        child: e["child_tag"].as_str().unwrap_or("").to_string(),
                    }).collect())
                    .unwrap_or_default(),
            };

            let toml_str = toml::to_string_pretty(&tree_file)?;

            if let Some(path) = output {
                std::fs::write(&path, &toml_str)
                    .with_context(|| format!("Cannot write {}", path.display()))?;
                println!("Exported to {}", path.display());
            } else {
                print!("{toml_str}");
            }
        }

        TreeCommand::Import { uri, file } => {
            let token = config.token()?;

            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let tree_file: TreeFile = toml::from_str(&content)
                .with_context(|| format!("Invalid TOML in {}", file.display()))?;

            // Get current edges
            let detail: serde_json::Value = client()
                .get(format!("{base}/skill-trees/by-uri"))
                .query(&[("uri", &uri)])
                .send().await?
                .error_for_status().context("Skill tree not found")?
                .json().await?;

            let current: std::collections::HashSet<TreeEdge> = detail["edges"].as_array()
                .map(|arr| arr.iter().map(|e| TreeEdge {
                    parent: e["parent_tag"].as_str().unwrap_or("").to_string(),
                    child: e["child_tag"].as_str().unwrap_or("").to_string(),
                }).collect())
                .unwrap_or_default();

            let desired: std::collections::HashSet<TreeEdge> = tree_file.edges.into_iter().collect();

            // Compute diff
            let to_add: Vec<&TreeEdge> = desired.difference(&current).collect();
            let to_remove: Vec<&TreeEdge> = current.difference(&desired).collect();

            if to_add.is_empty() && to_remove.is_empty() {
                println!("Already up to date.");
                return Ok(());
            }

            // Apply removals
            for e in &to_remove {
                client()
                    .post(format!("{base}/skill-trees/edges/remove"))
                    .bearer_auth(token)
                    .json(&serde_json::json!({
                        "tree_uri": uri,
                        "parent_tag": e.parent,
                        "child_tag": e.child,
                    }))
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to remove edge: {} -> {}", e.parent, e.child))?;
            }

            // Apply additions
            for e in &to_add {
                client()
                    .post(format!("{base}/skill-trees/edges"))
                    .bearer_auth(token)
                    .json(&serde_json::json!({
                        "tree_uri": uri,
                        "parent_tag": e.parent,
                        "child_tag": e.child,
                    }))
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to add edge: {} -> {}", e.parent, e.child))?;
            }

            println!("Synced: +{} added, -{} removed", to_add.len(), to_remove.len());
        }

        TreeCommand::AddEdge { uri, parent, child } => {
            let token = config.token()?;
            client()
                .post(format!("{base}/skill-trees/edges"))
                .bearer_auth(token)
                .json(&serde_json::json!({
                    "tree_uri": uri,
                    "parent_tag": parent,
                    "child_tag": child,
                }))
                .send().await?
                .error_for_status().context("Add edge failed")?;

            println!("Added: {parent} -> {child}");
        }

        TreeCommand::RmEdge { uri, parent, child } => {
            let token = config.token()?;
            client()
                .post(format!("{base}/skill-trees/edges/remove"))
                .bearer_auth(token)
                .json(&serde_json::json!({
                    "tree_uri": uri,
                    "parent_tag": parent,
                    "child_tag": child,
                }))
                .send().await?
                .error_for_status().context("Remove edge failed")?;

            println!("Removed: {parent} -> {child}");
        }

        TreeCommand::Fork { uri } => {
            let token = config.token()?;
            let resp: serde_json::Value = client()
                .post(format!("{base}/skill-trees/fork"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "uri": uri }))
                .send().await?
                .error_for_status().context("Fork failed")?
                .json().await?;

            let new_uri = resp["at_uri"].as_str().unwrap_or("?");
            let title = resp["title"].as_str().unwrap_or("?");
            println!("Forked: {title}");
            println!("URI: {new_uri}");
        }

        TreeCommand::Adopt { uri } => {
            let token = config.token()?;
            client()
                .post(format!("{base}/skill-trees/adopt"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "tree_uri": uri }))
                .send().await?
                .error_for_status().context("Adopt failed")?;

            println!("Adopted skill tree as active.");
        }
        TreeCommand::AddPrereq { from, to, prereq_type } => {
            let token = config.token()?;
            client()
                .post(format!("{base}/tag-prereqs"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "from_tag": from, "to_tag": to, "prereq_type": prereq_type }))
                .send().await?
                .error_for_status().context("Add prereq failed")?;
            println!("Added prereq: {from} -> {to} ({prereq_type})");
        }
        TreeCommand::RmPrereq { from, to } => {
            let token = config.token()?;
            client()
                .delete(format!("{base}/tag-prereqs"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "from_tag": from, "to_tag": to }))
                .send().await?
                .error_for_status().context("Remove prereq failed")?;
            println!("Removed prereq: {from} -> {to}");
        }
        TreeCommand::ListPrereqs => {
            let token = config.token()?;
            let resp: Vec<serde_json::Value> = client()
                .get(format!("{base}/tag-prereqs"))
                .bearer_auth(token)
                .send().await?
                .error_for_status().context("List prereqs failed")?
                .json().await?;
            if resp.is_empty() {
                println!("No prerequisite relationships defined.");
            } else {
                for e in &resp {
                    println!("{} -> {} ({})",
                        e["from_tag"].as_str().unwrap_or("?"),
                        e["to_tag"].as_str().unwrap_or("?"),
                        e["prereq_type"].as_str().unwrap_or("?"));
                }
                println!("\nTotal: {} prereqs", resp.len());
            }
        }
    }

    Ok(())
}

/// Validate that an HTML file is a content fragment, not a full page.
/// Rejects files containing <html>, <head>, <body>, or <script> tags.
fn parse_prereqs(raw: &[String]) -> Result<Vec<fx_core::models::ArticlePrereq>> {
    use fx_core::content::PrereqType;
    raw.iter().map(|s| {
        let (tag_id, prereq_type) = if let Some((t, p)) = s.split_once(':') {
            let pt = match p {
                "required" | "r" => PrereqType::Required,
                "recommended" | "rec" => PrereqType::Recommended,
                "suggested" | "s" => PrereqType::Suggested,
                _ => bail!("Invalid prereq type '{p}' (use required/recommended/suggested)"),
            };
            (t.to_string(), pt)
        } else {
            (s.clone(), PrereqType::Required)
        };
        Ok(fx_core::models::ArticlePrereq { tag_id, prereq_type })
    }).collect()
}

fn validate_html_fragment(content: &str) -> Result<()> {
    let lower = content.to_ascii_lowercase();
    let forbidden = [
        ("<!doctype", "<!DOCTYPE> declaration"),
        ("<html", "<html> tag"),
        ("<head", "<head> tag"),
        ("<body", "<body> tag"),
        ("<script", "<script> tag"),
    ];
    for (tag, label) in &forbidden {
        if lower.contains(tag) {
            bail!(
                "HTML file contains {label}.\n\
                 HTML articles should be content fragments (e.g. <h2>, <p>, <div>),\n\
                 not full HTML pages. Remove the page wrapper and try again.\n\
                 See: https://nightboat.dzming.li/#/guide for details."
            );
        }
    }
    Ok(())
}

/// Resolve a DID or handle to a DID. If already a DID (starts with "did:"), pass through.
/// Otherwise treat as a platform user handle and generate did:local:<handle>.
fn resolve_did_or_handle(input: &str) -> String {
    if input.starts_with("did:") {
        input.to_string()
    } else {
        format!("did:local:{input}")
    }
}

async fn handle_course(base: &str, config: &Config, action: CourseCommand) -> Result<()> {
    let token = config.token()?;
    match action {
        CourseCommand::List => {
            let resp: Vec<serde_json::Value> = client()
                .get(format!("{base}/courses"))
                .send().await?
                .error_for_status().context("List courses failed")?
                .json().await?;

            if resp.is_empty() {
                println!("No courses.");
            }
            for c in &resp {
                let id = c["id"].as_str().unwrap_or("?");
                let title = c["title"].as_str().unwrap_or("?");
                let code = c["code"].as_str().unwrap_or("");
                let inst = c["institution"].as_str().unwrap_or("");
                if code.is_empty() {
                    println!("{id}\t{title}\t{inst}");
                } else {
                    println!("{id}\t{code} {title}\t{inst}");
                }
            }
        }

        CourseCommand::Show { id } => {
            let resp: serde_json::Value = client()
                .get(format!("{base}/courses/{id}"))
                .send().await?
                .error_for_status().context("Get course failed")?
                .json().await?;

            let c = &resp["course"];
            let code = c["code"].as_str().unwrap_or("");
            let title = c["title"].as_str().unwrap_or("?");
            let inst = c["institution"].as_str().unwrap_or("");
            println!("{code} {title}  ({inst})");
            println!("ID: {id}");

            if let Some(sessions) = resp["sessions"].as_array() {
                if !sessions.is_empty() {
                    println!("\nSessions:");
                    for s in sessions {
                        let sid = s["id"].as_str().unwrap_or("?");
                        let order = s["sort_order"].as_i64().unwrap_or(0);
                        let topic = s["topic"].as_str().unwrap_or("-");
                        let readings = s["readings"].as_str().unwrap_or("");
                        let res = s["resources"].as_array();
                        let video = if res.map_or(false, |r| r.iter().any(|x| x["type"] == "video")) { "📹" } else { "" };
                        let notes = if res.map_or(false, |r| r.iter().any(|x| x["type"] == "notes")) { "📝" } else { "" };
                        let hw = if res.map_or(false, |r| r.iter().any(|x| x["type"] == "hw")) { "📋" } else { "" };
                        print!("  {order}. {topic}");
                        if !readings.is_empty() { print!("  [{readings}]"); }
                        if !video.is_empty() { print!(" {video}"); }
                        if !notes.is_empty() { print!(" {notes}"); }
                        if !hw.is_empty() { print!(" {hw}"); }

                        // Show tags
                        if let Some(tags) = s["tags"].as_array() {
                            if !tags.is_empty() {
                                let names: Vec<&str> = tags.iter()
                                    .filter_map(|t| t["tag_name"].as_str())
                                    .collect();
                                print!("  [{}]", names.join(", "));
                            }
                        }
                        println!("  ({sid})");
                    }
                }
            }

            if let Some(textbooks) = resp["textbooks"].as_array() {
                if !textbooks.is_empty() {
                    println!("\nTextbooks:");
                    for tb in textbooks {
                        let title = tb["title"].as_str().unwrap_or("?");
                        let role = tb["role"].as_str().unwrap_or("?");
                        println!("  - {title} ({role})");
                    }
                }
            }
        }

        CourseCommand::Create { title, code, desc, institution, department, semester, lang, source_url, source_attribution } => {
            let body = serde_json::json!({
                "title": title,
                "code": code,
                "description": desc,
                "institution": institution,
                "department": department,
                "semester": semester,
                "lang": lang,
                "source_url": source_url,
                "source_attribution": source_attribution,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/courses"))
                .bearer_auth(&token)
                .json(&body)
                .send().await?
                .error_for_status().context("Create course failed")?
                .json().await?;

            let id = resp["id"].as_str().unwrap_or("?");
            println!("Created course: {title}");
            println!("ID: {id}");
        }

        CourseCommand::Update { id, title, desc, publish } => {
            let mut body = serde_json::json!({});
            if let Some(t) = &title { body["title"] = serde_json::json!(t); }
            if let Some(d) = &desc { body["description"] = serde_json::json!(d); }
            if let Some(p) = publish { body["is_published"] = serde_json::json!(p); }

            client()
                .put(format!("{base}/courses/{id}"))
                .bearer_auth(&token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update course failed")?;

            println!("Updated course {id}");
        }

        CourseCommand::AddSession { course_id, topic, date, readings, resource, order, tags, prereqs } => {
            let resources: Vec<serde_json::Value> = resource.iter().filter_map(|s| {
                let parts: Vec<&str> = s.splitn(3, ':').collect();
                if parts.len() == 3 {
                    Some(serde_json::json!({"type": parts[0], "label": parts[1], "url": parts[2]}))
                } else { None }
            }).collect();

            let body = serde_json::json!({
                "topic": topic,
                "date": date,
                "readings": readings,
                "resources": resources,
                "sort_order": order,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/courses/{course_id}/sessions"))
                .bearer_auth(&token)
                .json(&body)
                .send().await?
                .error_for_status().context("Create session failed")?
                .json().await?;

            let session_id = resp["id"].as_str().unwrap_or("?");
            let topic_str = topic.as_deref().unwrap_or("(untitled)");
            println!("Created session: {topic_str} ({session_id})");

            // Add tags
            for tag_id in &tags {
                client()
                    .post(format!("{base}/courses/{course_id}/sessions/{session_id}/tags"))
                    .bearer_auth(&token)
                    .json(&serde_json::json!({ "tag_id": tag_id }))
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to add tag {tag_id}"))?;
            }
            if !tags.is_empty() {
                println!("  Added {} tags", tags.len());
            }

            // Add prereqs
            for tag_id in &prereqs {
                client()
                    .post(format!("{base}/courses/{course_id}/sessions/{session_id}/prereqs"))
                    .bearer_auth(&token)
                    .json(&serde_json::json!({ "tag_id": tag_id }))
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to add prereq {tag_id}"))?;
            }
            if !prereqs.is_empty() {
                println!("  Added {} prereqs", prereqs.len());
            }
        }

        CourseCommand::UpdateSession { course_id, session_id, topic, readings, resource } => {
            let mut body = serde_json::json!({
                "topic": topic,
                "readings": readings,
            });
            if !resource.is_empty() {
                let resources: Vec<serde_json::Value> = resource.iter().filter_map(|s| {
                    let parts: Vec<&str> = s.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        Some(serde_json::json!({"type": parts[0], "label": parts[1], "url": parts[2]}))
                    } else { None }
                }).collect();
                body["resources"] = serde_json::json!(resources);
            }

            client()
                .put(format!("{base}/courses/{course_id}/sessions/{session_id}"))
                .bearer_auth(&token)
                .json(&body)
                .send().await?
                .error_for_status().context("Update session failed")?;

            println!("Updated session {session_id}");
        }

        CourseCommand::RmSession { course_id, session_id } => {
            client()
                .delete(format!("{base}/courses/{course_id}/sessions/{session_id}"))
                .bearer_auth(&token)
                .send().await?
                .error_for_status().context("Delete session failed")?;

            println!("Deleted session {session_id}");
        }

        CourseCommand::AddTag { course_id, tag_id } => {
            client()
                .post(format!("{base}/courses/{course_id}/tags"))
                .bearer_auth(&token)
                .json(&serde_json::json!({ "tag_id": tag_id }))
                .send().await?
                .error_for_status().context("Add tag failed")?;

            println!("Added tag {tag_id} to course {course_id}");
        }

        CourseCommand::RmTag { course_id, tag_id } => {
            client()
                .delete(format!("{base}/courses/{course_id}/tags"))
                .bearer_auth(&token)
                .query(&[("tag_id", &tag_id)])
                .send().await?
                .error_for_status().context("Remove tag failed")?;

            println!("Removed tag {tag_id} from course {course_id}");
        }

        CourseCommand::AddTextbook { course_id, book_id, role } => {
            client()
                .post(format!("{base}/courses/{course_id}/textbooks"))
                .bearer_auth(&token)
                .json(&serde_json::json!({ "book_id": book_id, "role": role }))
                .send().await?
                .error_for_status().context("Add textbook failed")?;

            println!("Added textbook {book_id} to course {course_id}");
        }

        CourseCommand::Import { course_id, file } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let data: toml::Value = content.parse()
                .context("Invalid TOML")?;

            let sessions = data.get("session")
                .and_then(|v| v.as_array())
                .context("Expected [[session]] array in TOML")?;

            // Fetch existing sessions to enable incremental updates
            let detail: serde_json::Value = client()
                .get(format!("{base}/courses/{course_id}"))
                .send().await?
                .error_for_status().context("Failed to fetch course")?
                .json().await?;

            let existing: std::collections::HashMap<i64, serde_json::Value> = detail["sessions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|s| {
                    let order = s["sort_order"].as_i64()?;
                    Some((order, s.clone()))
                })
                .collect();

            let mut created = 0;
            let mut updated = 0;
            let mut skipped = 0;

            for (i, s) in sessions.iter().enumerate() {
                let sort_order = s.get("order").and_then(|v| v.as_integer()).unwrap_or((i + 1) as i64);

                // Build resources array from TOML
                let mut resources = Vec::new();
                if let Some(res_arr) = s.get("resources").and_then(|v| v.as_array()) {
                    // New format: [[session.resources]]
                    for r in res_arr {
                        resources.push(serde_json::json!({
                            "type": r.get("type").and_then(|v| v.as_str()).unwrap_or("notes"),
                            "url": r.get("url").and_then(|v| v.as_str()).unwrap_or(""),
                            "label": r.get("label").and_then(|v| v.as_str()).unwrap_or(""),
                        }));
                    }
                }

                let body = serde_json::json!({
                    "topic": s.get("topic").and_then(|v| v.as_str()),
                    "date": s.get("date").and_then(|v| v.as_str()),
                    "readings": s.get("readings").and_then(|v| v.as_str()),
                    "resources": resources,
                    "sort_order": sort_order,
                });

                let topic = s.get("topic").and_then(|v| v.as_str()).unwrap_or("-");

                if let Some(ex) = existing.get(&sort_order) {
                    // Check if anything changed
                    let changed = body["topic"] != ex["topic"]
                        || body["date"] != ex["date"]
                        || body["readings"] != ex["readings"]
                        || body["resources"] != ex["resources"];

                    if !changed {
                        skipped += 1;
                        continue;
                    }

                    // Update existing session
                    let session_id = ex["id"].as_str().unwrap_or("?");
                    client()
                        .put(format!("{base}/courses/{course_id}/sessions/{session_id}"))
                        .bearer_auth(&token)
                        .json(&body)
                        .send().await?
                        .error_for_status()
                        .with_context(|| format!("Failed to update session {sort_order}"))?;
                    println!("  [{}/{}] ~ {topic} (updated)", i + 1, sessions.len());
                    updated += 1;
                } else {
                    // Create new session
                    let resp: serde_json::Value = client()
                        .post(format!("{base}/courses/{course_id}/sessions"))
                        .bearer_auth(&token)
                        .json(&body)
                        .send().await?
                        .error_for_status()
                        .with_context(|| format!("Failed to create session {}", i + 1))?
                        .json().await?;

                    let session_id = resp["id"].as_str().unwrap_or("?");
                    println!("  [{}/{}] + {topic} ({session_id})", i + 1, sessions.len());
                    created += 1;

                    // Add tags
                    if let Some(tags) = s.get("tags").and_then(|v| v.as_array()) {
                        for tag in tags {
                            if let Some(tag_id) = tag.as_str() {
                                client()
                                    .post(format!("{base}/courses/{course_id}/sessions/{session_id}/tags"))
                                    .bearer_auth(&token)
                                    .json(&serde_json::json!({ "tag_id": tag_id }))
                                    .send().await?
                                    .error_for_status()?;
                            }
                        }
                    }

                    // Add prereqs
                    if let Some(prereqs) = s.get("prereqs").and_then(|v| v.as_array()) {
                        for tag in prereqs {
                            if let Some(tag_id) = tag.as_str() {
                                client()
                                    .post(format!("{base}/courses/{course_id}/sessions/{session_id}/prereqs"))
                                    .bearer_auth(&token)
                                    .json(&serde_json::json!({ "tag_id": tag_id }))
                                    .send().await?
                                    .error_for_status()?;
                            }
                        }
                    }
                }
            }

            println!("\n{created} created, {updated} updated, {skipped} unchanged");
        }
    }

    Ok(())
}

async fn handle_admin(base: &str, config: &mut Config, action: AdminCommand) -> Result<()> {
    let secret = std::env::var("NBT_ADMIN_SECRET")
        .ok()
        .or_else(|| std::env::var("FX_ADMIN_SECRET").ok())
        .or_else(|| config.admin_secret.clone())
        .context("Admin secret not set. Use NBT_ADMIN_SECRET env var")?;

    match action {
        AdminCommand::CreateUser { handle, password, display_name } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/platform-users"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({
                    "handle": handle,
                    "password": password,
                    "display_name": display_name,
                }))
                .send().await?
                .error_for_status().context("Create user failed")?
                .json().await?;

            let did = resp["did"].as_str().unwrap_or("?");
            println!("Created: {handle} ({did})");
        }

        AdminCommand::ListUsers => {
            let users: Vec<serde_json::Value> = client()
                .get(format!("{base}/admin/platform-users"))
                .header("x-admin-secret", &secret)
                .send().await?
                .error_for_status().context("List users failed")?
                .json().await?;

            if users.is_empty() {
                println!("No platform users.");
            }
            for u in &users {
                let handle = u["handle"].as_str().unwrap_or("?");
                let did = u["did"].as_str().unwrap_or("?");
                let name = u["display_name"].as_str().unwrap_or("");
                println!("{handle}\t{did}\t{name}");
            }
        }

        AdminCommand::SetTagName { id, locale, name } => {
            // First get existing names
            let tag: serde_json::Value = client()
                .get(format!("{base}/tags/by-id"))
                .query(&[("id", &id)])
                .send().await?
                .error_for_status().context("Tag not found")?
                .json().await?;

            let mut names: std::collections::HashMap<String, String> = tag["names"]
                .as_object()
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                .unwrap_or_default();

            names.insert(locale.clone(), name.clone());

            let resp: serde_json::Value = client()
                .post(format!("{base}/tags/names"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "id": id, "names": names }))
                .send().await?
                .error_for_status().context("Update tag names failed")?
                .json().await?;

            let updated_names = &resp["names"];
            println!("Updated tag '{id}': {updated_names}");
        }

        AdminCommand::AddTagAlias { id, alias } => {
            client()
                .post(format!("{base}/admin/tags/alias"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "tag_id": id, "alias": alias }))
                .send().await?
                .error_for_status().context("Add alias failed")?;
            println!("Added alias '{alias}' -> tag '{id}'");
        }

        AdminCommand::RmTagAlias { alias } => {
            client()
                .delete(format!("{base}/admin/tags/alias"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "alias": alias }))
                .send().await?
                .error_for_status().context("Remove alias failed")?;
            println!("Removed alias '{alias}'");
        }

        AdminCommand::MergeTag { from, into } => {
            client()
                .post(format!("{base}/admin/tags/merge"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "from": from, "into": into }))
                .send().await?
                .error_for_status().context("Merge tag failed")?;

            println!("Merged tag '{from}' into '{into}'");
        }

        AdminCommand::MergeQuestions { from, into } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/questions/merge"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "from_uri": from, "into_uri": into }))
                .send().await?
                .error_for_status().context("Merge questions failed")?
                .json().await?;

            let moved = resp.get("answers_moved").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("Merged question into '{into}' ({moved} answers moved)");
        }

        AdminCommand::CreateSeries { r#as: as_handle, title, desc, topics, parent, lang, translation_of } => {
            let topics_vec: Vec<&str> = topics.as_deref().map(|t| t.split(',').collect()).unwrap_or_default();
            let body = serde_json::json!({
                "as_handle": as_handle,
                "title": title,
                "description": desc,
                "topics": topics_vec,
                "parent_id": parent,
                "lang": lang,
                "translation_of": translation_of,
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/series"))
                .header("x-admin-secret", &secret)
                .json(&body)
                .send().await?
                .error_for_status().context("Create series failed")?
                .json().await?;

            let id = resp["id"].as_str().unwrap_or("?");
            println!("Created series: {title}");
            println!("ID: {id}");
        }

        AdminCommand::UploadSeriesCover { id, file } => {
            let file_bytes = std::fs::read(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;
            let file_name = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("cover.jpg");
            let part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string());
            let form = reqwest::multipart::Form::new().part("file", part);

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/series/cover"))
                .query(&[("id", &id)])
                .header("x-admin-secret", &secret)
                .multipart(form)
                .send().await?
                .error_for_status().context("Upload series cover failed")?
                .json().await?;

            let url = resp["cover_url"].as_str().unwrap_or("?");
            println!("Uploaded cover for series {id}");
            println!("URL: {url}");
        }

        AdminCommand::SetSeriesCoverRef { id, file } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/series/cover/reference"))
                .query(&[("id", &id)])
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "file": file }))
                .send().await?
                .error_for_status().context("Set series cover reference failed")?
                .json().await?;

            let url = resp["cover_url"].as_str().unwrap_or("?");
            let f = resp["cover_file"].as_str().unwrap_or("?");
            println!("Set cover for series {id} → {f}");
            println!("URL: {url}");
        }

        AdminCommand::AddToSeries { series, article } => {
            client()
                .post(format!("{base}/admin/series/articles"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({
                    "series_id": series,
                    "article_uri": article,
                }))
                .send().await?
                .error_for_status().context("Add to series failed")?;

            println!("Added {article} to series {series}");
        }

        AdminCommand::Update { uri, file, title, desc } => {
            let mut body = serde_json::json!({ "uri": uri });
            if let Some(ref t) = title {
                body["title"] = serde_json::Value::String(t.clone());
            }
            if let Some(ref d) = desc {
                body["description"] = serde_json::Value::String(d.clone());
            }
            if let Some(ref f) = file {
                let content = std::fs::read_to_string(f)
                    .with_context(|| format!("Cannot read {}", f.display()))?;
                if f.extension().and_then(|e| e.to_str()) == Some("html") {
                    validate_html_fragment(&content)?;
                }
                body["content"] = serde_json::Value::String(content);
            }

            client()
                .post(format!("{base}/admin/articles/update"))
                .header("x-admin-secret", &secret)
                .json(&body)
                .send().await?
                .error_for_status().context("Admin update failed")?;

            println!("Updated article: {uri}");
        }

        AdminCommand::BanUser { did_or_handle, reason } => {
            let did = resolve_did_or_handle(&did_or_handle);
            client()
                .post(format!("{base}/admin/ban-user"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "did": did, "reason": reason }))
                .send().await?
                .error_for_status().context("Ban user failed")?;

            println!("Banned: {did_or_handle} ({did})");
        }

        AdminCommand::UnbanUser { did_or_handle } => {
            let did = resolve_did_or_handle(&did_or_handle);
            client()
                .post(format!("{base}/admin/unban-user"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "did": did }))
                .send().await?
                .error_for_status().context("Unban user failed")?;

            println!("Unbanned: {did_or_handle} ({did})");
        }

        AdminCommand::BannedUsers => {
            let users: Vec<serde_json::Value> = client()
                .get(format!("{base}/admin/banned-users"))
                .header("x-admin-secret", &secret)
                .send().await?
                .error_for_status().context("List banned users failed")?
                .json().await?;

            if users.is_empty() {
                println!("No banned users.");
            }
            for u in &users {
                let handle = u["handle"].as_str().unwrap_or("?");
                let did = u["did"].as_str().unwrap_or("?");
                let reason = u["ban_reason"].as_str().unwrap_or("-");
                let at = u["banned_at"].as_str().unwrap_or("?");
                println!("{handle}\t{did}\t{at}\t{reason}");
            }
        }

        AdminCommand::DeleteArticle { uri, reason } => {
            client()
                .post(format!("{base}/admin/articles/delete"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "uri": uri, "reason": reason }))
                .send().await?
                .error_for_status().context("Delete article failed")?;

            println!("Soft-deleted (30-day appeal window): {uri}");
        }

        AdminCommand::SetVisibility { uri, visibility, reason } => {
            client()
                .post(format!("{base}/admin/articles/visibility"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "uri": uri, "visibility": visibility, "reason": reason }))
                .send().await?
                .error_for_status().context("Set visibility failed")?;

            println!("Set visibility to '{visibility}': {uri}");
        }

        AdminCommand::Appeals => {
            let appeals: Vec<serde_json::Value> = client()
                .get(format!("{base}/admin/appeals"))
                .header("x-admin-secret", &secret)
                .send().await?
                .error_for_status().context("List appeals failed")?
                .json().await?;

            if appeals.is_empty() {
                println!("No pending appeals.");
            }
            for a in &appeals {
                let id = a["id"].as_str().unwrap_or("?");
                let did = a["did"].as_str().unwrap_or("?");
                let kind = a["kind"].as_str().unwrap_or("?");
                let reason = a["reason"].as_str().unwrap_or("-");
                let target = a["target_uri"].as_str().unwrap_or("-");
                let at = a["created_at"].as_str().unwrap_or("?");
                println!("[{id}] {kind}\t{did}\t{at}");
                if target != "-" {
                    println!("  target: {target}");
                }
                println!("  reason: {reason}");
                println!();
            }
        }

        AdminCommand::ResolveAppeal { id, status, response } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/appeals/resolve"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({
                    "id": id,
                    "status": status,
                    "response": response,
                }))
                .send().await?
                .error_for_status().context("Resolve appeal failed")?
                .json().await?;

            let kind = resp["kind"].as_str().unwrap_or("?");
            let did = resp["did"].as_str().unwrap_or("?");
            println!("Appeal {id} ({kind} by {did}): {status}");
        }

        AdminCommand::PublishQuestion { r#as: as_handle, file, title, desc, lang, tags } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;

            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            let (content_format, content) = match ext {
                "md" | "markdown" => ("markdown", content),
                "typ" | "typst" => ("typst", content),
                "html" | "htm" => ("html", content),
                _ => bail!("Unsupported file extension: .{ext} (use .md, .typ, or .html)"),
            };

            let title = title.unwrap_or_else(|| {
                file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

            let body = serde_json::json!({
                "as_handle": as_handle,
                "title": title,
                "description": desc.unwrap_or_default(),
                "content": content,
                "content_format": content_format,
                "lang": lang,
                "tags": tags,
                "prereqs": [],
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/questions"))
                .header("x-admin-secret", &secret)
                .json(&body)
                .send().await?
                .error_for_status().context("Publish question failed")?
                .json().await?;

            let uri = resp["at_uri"].as_str().unwrap_or("?");
            println!("Published question as {as_handle}: {title}");
            println!("URI: {uri}");
        }

        AdminCommand::PublishAnswer { r#as: as_handle, question, file, title, desc, lang } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;

            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            let (content_format, content) = match ext {
                "md" | "markdown" => ("markdown", content),
                "typ" | "typst" => ("typst", content),
                "html" | "htm" => ("html", content),
                _ => bail!("Unsupported file extension: .{ext} (use .md, .typ, or .html)"),
            };

            let title = title.unwrap_or_else(|| {
                file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Answer")
                    .to_string()
            });

            let body = serde_json::json!({
                "as_handle": as_handle,
                "question_uri": question,
                "title": title,
                "description": desc.unwrap_or_default(),
                "content": content,
                "content_format": content_format,
                "lang": lang,
                "tags": [],
                "prereqs": [],
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/questions/answer"))
                .header("x-admin-secret", &secret)
                .json(&body)
                .send().await?
                .error_for_status().context("Publish answer failed")?
                .json().await?;

            let uri = resp["at_uri"].as_str().unwrap_or("?");
            println!("Published answer as {as_handle}: {title}");
            println!("URI: {uri}");
        }

        AdminCommand::VerifyCredentials { did_or_handle, education, affiliation } => {
            let did = resolve_did_or_handle(&did_or_handle);
            let education_val: serde_json::Value = if let Some(ref e) = education {
                serde_json::from_str(e).context("Invalid JSON for --education")?
            } else {
                serde_json::json!([])
            };

            client()
                .post(format!("{base}/admin/credentials/verify"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({
                    "did": did,
                    "education": education_val,
                    "affiliation": affiliation,
                }))
                .send().await?
                .error_for_status().context("Verify credentials failed")?;

            println!("Verified credentials for {did_or_handle} ({did})");
        }

        AdminCommand::RevokeCredentials { did_or_handle } => {
            let did = resolve_did_or_handle(&did_or_handle);
            client()
                .post(format!("{base}/admin/credentials/revoke"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "did": did }))
                .send().await?
                .error_for_status().context("Revoke credentials failed")?;

            println!("Revoked credentials for {did_or_handle} ({did})");
        }

        AdminCommand::Publish { r#as: as_handle, file, title, desc, lang, tags, license, translation_of, category, book_id, series, resource } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read {}", file.display()))?;

            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            let (content_format, content) = match ext {
                "md" | "markdown" => ("markdown", content),
                "typ" | "typst" => ("typst", content),
                "html" | "htm" => ("html", content),
                _ => bail!("Unsupported file extension: .{ext} (use .md, .typ, or .html)"),
            };

            if content_format == "html" {
                validate_html_fragment(&content)?;
            }

            let title = title.unwrap_or_else(|| {
                file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

            let body = serde_json::json!({
                "as_handle": as_handle,
                "title": title,
                "description": desc.unwrap_or_default(),
                "content": content,
                "content_format": content_format,
                "lang": lang,
                "license": license,
                "translation_of": translation_of,
                "category": category,
                "book_id": book_id,
                "series_id": series,
                "tags": tags,
                "prereqs": [],
            });

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/articles"))
                .header("x-admin-secret", &secret)
                .json(&body)
                .send().await?
                .error_for_status().context("Publish failed")?
                .json().await?;

            let uri = resp["at_uri"].as_str().unwrap_or("?");
            println!("Published as {as_handle}: {title}");
            println!("URI: {uri}");

            // Add to series if specified
            if let Some(ref series_id) = series {
                client()
                    .post(format!("{base}/series/{series_id}/articles"))
                    .header("x-admin-secret", &secret)
                    .json(&serde_json::json!({ "article_uri": uri }))
                    .send().await?
                    .error_for_status().context("Failed to add article to series")?;
                println!("Added to series: {series_id}");
            }

            // Upload resource files
            for res_path in &resource {
                let file_name = res_path.file_name()
                    .and_then(|n| n.to_str())
                    .context("Invalid resource filename")?;
                let file_bytes = std::fs::read(res_path)
                    .with_context(|| format!("Cannot read {}", res_path.display()))?;

                if let Some(ref series_id) = series {
                    let part = reqwest::multipart::Part::bytes(file_bytes)
                        .file_name(file_name.to_string());
                    let form = reqwest::multipart::Form::new().part("file", part);
                    client()
                        .post(format!("{base}/series/{series_id}/resource"))
                        .header("x-admin-secret", &secret)
                        .multipart(form)
                        .send().await?
                        .error_for_status()
                        .with_context(|| format!("Failed to upload resource: {file_name}"))?;
                    println!("Uploaded to series: {file_name}");
                } else {
                    let part = reqwest::multipart::Part::bytes(file_bytes)
                        .file_name(file_name.to_string());
                    let form = reqwest::multipart::Form::new()
                        .text("article_uri", uri.to_string())
                        .part("file", part);
                    client()
                        .post(format!("{base}/admin/articles/upload-image"))
                        .header("x-admin-secret", &secret)
                        .multipart(form)
                        .send().await?
                        .error_for_status()
                        .with_context(|| format!("Failed to upload resource: {file_name}"))?;
                    println!("Uploaded to article: {file_name}");
                }
            }
        }

        AdminCommand::BookHistory { book_id } => {
            let resp: Vec<serde_json::Value> = client()
                .get(format!("{base}/books/history"))
                .query(&[("id", &book_id)])
                .header("x-admin-secret", &secret)
                .send().await?
                .error_for_status().context("Failed to get book history")?
                .json().await?;

            if resp.is_empty() {
                println!("No edit history for {book_id}");
            } else {
                for entry in &resp {
                    let id = entry["id"].as_str().unwrap_or("?");
                    let editor = entry["editor_handle"].as_str()
                        .unwrap_or(entry["editor_did"].as_str().unwrap_or("?"));
                    let summary = entry["summary"].as_str().unwrap_or("");
                    let time = entry["created_at"].as_str().unwrap_or("?");
                    println!("[{id}] {editor}: {summary} ({time})");
                }
            }
        }

        AdminCommand::RevertBookEdit { edit_id } => {
            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/books/revert-edit"))
                .header("x-admin-secret", &secret)
                .json(&serde_json::json!({ "edit_id": edit_id }))
                .send().await?
                .error_for_status().context("Revert failed")?
                .json().await?;

            let book_id = resp["book_id"].as_str().unwrap_or("?");
            println!("Reverted edit {edit_id} on book {book_id}");
        }

        AdminCommand::ImportDir { r#as: as_handle, series, dir, lang, license, tags, dry_run } => {
            // Scan directory for chapter subdirectories
            let mut chapters: Vec<(String, PathBuf, String)> = Vec::new(); // (dir_name, content_file, format)
            let mut resources: Vec<(String, PathBuf)> = Vec::new(); // (relative_path, absolute_path)

            let root = dir.canonicalize().context("Cannot resolve directory")?;
            let mut entries: Vec<_> = std::fs::read_dir(&root)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            entries.sort_by_key(|e| e.file_name());

            for entry in &entries {
                let sub_dir = entry.path();
                let dir_name = entry.file_name().to_string_lossy().to_string();

                // Look for index.md, index.typ, or single content file
                let content_file = ["index.md", "index.typ", "index.html"]
                    .iter()
                    .map(|f| sub_dir.join(f))
                    .find(|p| p.exists());

                let content_file = if let Some(f) = content_file {
                    f
                } else {
                    // Try any single .md/.typ file in the directory
                    let mut files: Vec<_> = std::fs::read_dir(&sub_dir)?
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            let ext = e.path().extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
                            matches!(ext.as_str(), "md" | "typ" | "html")
                        })
                        .collect();
                    if files.len() == 1 {
                        files.remove(0).path()
                    } else {
                        println!("  Skipping {dir_name}: no content file found");
                        continue;
                    }
                };

                let ext = content_file.extension().and_then(|e| e.to_str()).unwrap_or("md");
                let format = match ext {
                    "typ" => "typst",
                    "html" => "html",
                    _ => "markdown",
                };

                chapters.push((dir_name.clone(), content_file, format.to_string()));

                // Collect all non-content files as resources
                for res in walkdir::WalkDir::new(&sub_dir).into_iter().filter_map(|e| e.ok()) {
                    if !res.file_type().is_file() { continue; }
                    let res_path = res.path();
                    let res_ext = res_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if matches!(res_ext, "md" | "typ" | "html") { continue; }
                    let rel = res_path.strip_prefix(&root).unwrap_or(res_path);
                    resources.push((rel.to_string_lossy().to_string(), res_path.to_path_buf()));
                }
            }

            println!("Found {} chapters, {} resource files in {}", chapters.len(), resources.len(), root.display());
            for (i, (name, _file, fmt)) in chapters.iter().enumerate() {
                println!("  Chapter {}: {} ({})", i + 1, name, fmt);
            }
            if !resources.is_empty() {
                println!("  Resources: {} files", resources.len());
            }

            if dry_run {
                println!("\n[dry run] No changes made.");
                return Ok(());
            }

            // Publish each chapter via admin API
            let mut published: Vec<(String, String)> = Vec::new();
            for (i, (dir_name, content_file, format)) in chapters.iter().enumerate() {
                let content = std::fs::read_to_string(content_file)
                    .with_context(|| format!("Cannot read {}", content_file.display()))?;

                // Derive title from directory name
                let title = dir_name
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '_')
                    .replace('-', " ")
                    .replace('_', " ");
                let title = if title.is_empty() { dir_name.clone() } else {
                    let mut chars = title.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                        None => title,
                    }
                };

                let body = serde_json::json!({
                    "as_handle": as_handle,
                    "title": title,
                    "description": "",
                    "content": content,
                    "content_format": format,
                    "lang": lang,
                    "license": license,
                    "category": "lecture",
                    "series_id": series,
                    "tags": tags,
                    "prereqs": [],
                });

                let resp: serde_json::Value = client()
                    .post(format!("{base}/admin/articles"))
                    .header("x-admin-secret", &secret)
                    .json(&body)
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to publish: {dir_name}"))?
                    .json().await?;

                let uri = resp["at_uri"].as_str().unwrap_or("?").to_string();
                println!("  [{}/{}] {} -> {}", i + 1, chapters.len(), dir_name, uri);

                // Add to series
                client()
                    .post(format!("{base}/series/{series}/articles"))
                    .header("x-admin-secret", &secret)
                    .json(&serde_json::json!({ "article_uri": uri }))
                    .send().await?
                    .error_for_status()
                    .context("Failed to add to series")?;

                published.push((dir_name.clone(), uri));
            }

            // Upload resources
            let mut uploaded = 0;
            for (rel_path, abs_path) in &resources {
                let file_bytes = std::fs::read(abs_path)
                    .with_context(|| format!("Cannot read {}", abs_path.display()))?;
                let file_name = abs_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file");

                let part = reqwest::multipart::Part::bytes(file_bytes)
                    .file_name(file_name.to_string());
                let form = reqwest::multipart::Form::new()
                    .text("path", rel_path.clone())
                    .part("file", part);

                client()
                    .post(format!("{base}/series/{series}/resource"))
                    .header("x-admin-secret", &secret)
                    .multipart(form)
                    .send().await?
                    .error_for_status()
                    .with_context(|| format!("Failed to upload: {rel_path}"))?;
                uploaded += 1;
            }
            if uploaded > 0 {
                println!("Uploaded {} resource files", uploaded);
            }

            println!("\nImported {} chapters into series {}", published.len(), series);
        }

        AdminCommand::ImportRepo { r#as: as_handle, series, dir, manifest, lang, license, tags, image_dirs, dry_run } => {
            use base64::Engine;
            use std::collections::HashSet;

            let root = dir.canonicalize().context("Cannot resolve directory")?;
            let manifest_content = std::fs::read_to_string(&manifest)
                .with_context(|| format!("Cannot read manifest {}", manifest.display()))?;
            let manifest_data: toml::Value = manifest_content.parse().context("Invalid TOML manifest")?;

            let article_entries = manifest_data.get("article")
                .and_then(|v| v.as_array())
                .context("Expected [[article]] array in manifest")?;

            // Build articles list
            let mut articles = Vec::new();
            let mut referenced_images: HashSet<String> = HashSet::new();

            for entry in article_entries {
                let title = entry.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                let path = entry.get("path").and_then(|v| v.as_str()).context("article missing 'path'")?;

                let full_path = root.join(path);
                let content = std::fs::read_to_string(&full_path)
                    .with_context(|| format!("Cannot read {}", full_path.display()))?;

                let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("md");
                let format = match ext {
                    "typ" | "typst" => "typst",
                    "html" | "htm" => "html",
                    _ => "markdown",
                };

                // Collect image references from this file
                let file_dir = std::path::Path::new(path).parent().unwrap_or(std::path::Path::new(""));
                for cap in regex_lite::Regex::new(r#"src="([^"]+\.(png|jpg|gif|svg))""#).unwrap().captures_iter(&content) {
                    let src = cap.get(1).unwrap().as_str();
                    if !src.starts_with("http") {
                        let src = src.strip_prefix("./").unwrap_or(src);
                        referenced_images.insert(file_dir.join(src).to_string_lossy().to_string());
                    }
                }
                for cap in regex_lite::Regex::new(r"!\[[^\]]*\]\(([^)]+\.(png|jpg|gif|svg))\)").unwrap().captures_iter(&content) {
                    let src = cap.get(1).unwrap().as_str();
                    if !src.starts_with("http") {
                        let src = src.strip_prefix("./").unwrap_or(src);
                        referenced_images.insert(file_dir.join(src).to_string_lossy().to_string());
                    }
                }

                let entry_tags: Vec<String> = entry.get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_else(|| tags.clone());

                articles.push(serde_json::json!({
                    "title": title,
                    "content": content,
                    "content_format": format,
                    "path": path,
                    "tags": entry_tags,
                    "license": entry.get("license").and_then(|v| v.as_str()).unwrap_or(&license),
                }));
            }

            // Collect images from referenced paths + extra image_dirs
            let mut all_image_paths: HashSet<String> = referenced_images;

            for img_dir in &image_dirs {
                let abs_dir = root.join(img_dir);
                if abs_dir.is_dir() {
                    for entry in walkdir::WalkDir::new(&abs_dir).into_iter().filter_map(|e| e.ok()) {
                        if !entry.file_type().is_file() { continue; }
                        let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
                        if matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp") {
                            let rel = entry.path().strip_prefix(&root).unwrap_or(entry.path());
                            all_image_paths.insert(rel.to_string_lossy().to_string());
                        }
                    }
                }
            }

            // Build files list (base64 encoded)
            let mut files = Vec::new();
            for img_path in &all_image_paths {
                let abs = root.join(img_path);
                if !abs.exists() { continue; }
                let data = std::fs::read(&abs)
                    .with_context(|| format!("Cannot read image {}", abs.display()))?;
                files.push(serde_json::json!({
                    "path": img_path,
                    "data": base64::engine::general_purpose::STANDARD.encode(&data),
                }));
            }

            println!("Articles: {}", articles.len());
            println!("Images: {}", files.len());

            if dry_run {
                for a in &articles {
                    println!("  {} -> {}", a["path"].as_str().unwrap_or("?"), a["title"].as_str().unwrap_or("?"));
                }
                println!("\n[dry run] No changes made.");
                return Ok(());
            }

            let body = serde_json::json!({
                "as_handle": as_handle,
                "series_id": series,
                "articles": articles,
                "files": files,
                "lang": lang,
            });

            let payload = serde_json::to_string(&body)?;
            let payload_mb = payload.len() as f64 / 1024.0 / 1024.0;
            println!("Payload: {payload_mb:.1} MB");

            // Write to temp file to avoid CLI length limits
            let tmp = std::env::temp_dir().join("fx-import-repo.json");
            std::fs::write(&tmp, &payload)?;

            let resp: serde_json::Value = client()
                .post(format!("{base}/admin/series/batch-publish"))
                .header("x-admin-secret", &secret)
                .header("content-type", "application/json")
                .body(std::fs::read(&tmp)?)
                .send().await?
                .error_for_status().context("Batch publish failed")?
                .json().await?;

            let count = resp.as_array().map(|a| a.len()).unwrap_or(0);
            println!("\nPublished {count} articles into series {series}");
        }
    }

    Ok(())
}
