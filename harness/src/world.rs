//! An in-memory [`World`] for compiling test documents.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Dict, Duration, IntoValue, Str};
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt, World, WorldExt};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;

use crate::repo_root;

/// Fonts shared by all compilations: only the fonts embedded in Typst.
static FONTS: LazyLock<FontStore> = LazyLock::new(|| {
    let mut store = FontStore::new();
    store.extend(typst_kit::fonts::embedded());
    store
});

/// A `sys.inputs` entry.
pub type Input<'a> = (&'a str, &'a str);

/// A successfully compiled document.
pub struct Compiled {
    pub doc: PagedDocument,
    pub warnings: Vec<String>,
}

/// Compiles Typst source text as if it were the file at `vpath` (relative to
/// the repository root, e.g. `tests/geometry/case.typ`).
pub fn compile(vpath: &str, text: &str, inputs: &[Input]) -> Result<Compiled, Vec<String>> {
    let world = TestWorld::new(vpath, text.to_string(), inputs);
    let warned = typst::compile::<PagedDocument>(&world);
    let warnings = warned.warnings.iter().map(|d| world.format(d)).collect();
    match warned.output {
        Ok(doc) => Ok(Compiled { doc, warnings }),
        Err(errors) => Err(errors.iter().map(|d| world.format(d)).collect()),
    }
}

/// Compiles a file from the repository (path relative to the repository root).
pub fn compile_file(vpath: &str, inputs: &[Input]) -> Result<Compiled, Vec<String>> {
    let text = std::fs::read_to_string(repo_root().join(vpath))
        .unwrap_or_else(|err| panic!("cannot read {vpath}: {err}"));
    compile(vpath, &text, inputs)
}

struct TestWorld {
    library: LazyHash<Library>,
    main: FileId,
    main_source: Source,
    sources: Mutex<HashMap<FileId, Source>>,
}

impl TestWorld {
    fn new(vpath: &str, text: String, inputs: &[Input]) -> Self {
        Self::with_features(vpath, text, inputs, false)
    }

    fn with_features(vpath: &str, text: String, inputs: &[Input], html: bool) -> Self {
        let mut dict = Dict::new();
        for (key, value) in inputs {
            dict.insert(Str::from(*key), Str::from(*value).into_value());
        }
        let mut builder = Library::builder().with_inputs(dict);
        if html {
            builder = builder.with_features([Feature::Html].into_iter().collect());
        }
        let library = builder.build();
        let path = VirtualPath::new(vpath).expect("valid virtual path");
        // A unique id per compilation: tests run in parallel and may compile
        // different sources under the same path.
        let main = FileId::unique(RootedPath::new(VirtualRoot::Project, path));
        Self {
            library: LazyHash::new(library),
            main,
            main_source: Source::new(main, text),
            sources: Mutex::new(HashMap::new()),
        }
    }

    /// The directory on disk that a file id lives in.
    fn root_dir(&self, id: FileId) -> FileResult<PathBuf> {
        match id.get().root() {
            VirtualRoot::Project => Ok(repo_root().to_path_buf()),
            VirtualRoot::Package(spec) => package_dir(spec),
        }
    }

    fn read(&self, id: FileId) -> FileResult<Vec<u8>> {
        let dir = self.root_dir(id)?;
        let path = id.get().vpath().realize(&dir).map_err(|_| FileError::AccessDenied)?;
        std::fs::read(&path).map_err(|err| FileError::from_io(err, &path))
    }

    fn format(&self, diag: &SourceDiagnostic) -> String {
        let kind = match diag.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let mut out = format!("{kind}: {}", diag.message);
        if let Some(id) = diag.span.id() {
            if let (Some(range), Ok(source)) = (self.range(diag.span), self.source(id)) {
                let lines = source.lines();
                let line = lines.byte_to_line(range.start).unwrap_or(0);
                let col = lines.byte_to_column(range.start).unwrap_or(0);
                out += &format!(" ({:?}:{}:{})", id.get().vpath(), line + 1, col + 1);
            }
        }
        for hint in &diag.hints {
            out += &format!("\n  hint: {}", hint.v);
        }
        out
    }
}

impl World for TestWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        FONTS.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main {
            return Ok(self.main_source.clone());
        }
        let mut sources = self.sources.lock().unwrap();
        if let Some(source) = sources.get(&id) {
            return Ok(source.clone());
        }
        let bytes = self.read(id)?;
        let text = String::from_utf8(bytes).map_err(|_| FileError::InvalidUtf8)?;
        let source = Source::new(id, text);
        sources.insert(id, source.clone());
        Ok(source)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.read(id).map(Bytes::new)
    }

    fn font(&self, index: usize) -> Option<Font> {
        FONTS.font(index)
    }

    fn today(&self, _: Option<Duration>) -> Option<Datetime> {
        Datetime::from_ymd(2026, 1, 1)
    }
}

/// Resolves a package to a directory on disk.
///
/// Only `@preview/runaround:<current version>` resolves, to the working tree,
/// so README examples are tested against the code being developed. Tests never
/// depend on other packages.
fn package_dir(spec: &PackageSpec) -> FileResult<PathBuf> {
    if spec.namespace == "preview"
        && spec.name == "runaround"
        && spec.version.to_string() == crate::package_version()
    {
        return Ok(repo_root().to_path_buf());
    }
    Err(FileError::Other(Some(format!("package {spec} is not available in tests").into())))
}

/// Compiles a file from the repository to HTML (experimental HTML export).
/// Returns the HTML and the warnings.
pub fn compile_html(vpath: &str) -> Result<(String, Vec<String>), Vec<String>> {
    let text = std::fs::read_to_string(repo_root().join(vpath))
        .unwrap_or_else(|err| panic!("cannot read {vpath}: {err}"));
    let world = TestWorld::with_features(vpath, text, &[], true);
    let warned = typst::compile::<typst_html::HtmlDocument>(&world);
    let warnings = warned.warnings.iter().map(|d| world.format(d)).collect();
    let doc = warned
        .output
        .map_err(|errors| errors.iter().map(|d| world.format(d)).collect::<Vec<_>>())?;
    let html = typst_html::html(&doc, &typst_html::HtmlOptions { pretty: false })
        .map_err(|errors| errors.iter().map(|d| world.format(d)).collect::<Vec<_>>())?;
    Ok((html, warnings))
}
