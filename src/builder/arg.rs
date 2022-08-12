// Std
use std::{
    borrow::Cow,
    cmp::{Ord, Ordering},
    ffi::OsStr,
    fmt::{self, Display, Formatter},
    str,
};
#[cfg(feature = "env")]
use std::{env, ffi::OsString};

// Internal
use super::{ArgFlags, ArgSettings};
use crate::builder::ArgPredicate;
use crate::builder::PossibleValue;
use crate::builder::ValueRange;
use crate::util::{Id, Key};
use crate::ArgAction;
use crate::ValueHint;
use crate::INTERNAL_ERROR_MSG;

/// The abstract representation of a command line argument. Used to set all the options and
/// relationships that define a valid argument for the program.
///
/// There are two methods for constructing [`Arg`]s, using the builder pattern and setting options
/// manually, or using a usage string which is far less verbose but has fewer options. You can also
/// use a combination of the two methods to achieve the best of both worlds.
///
/// - [Basic API][crate::Arg#basic-api]
/// - [Value Handling][crate::Arg#value-handling]
/// - [Help][crate::Arg#help-1]
/// - [Advanced Argument Relations][crate::Arg#advanced-argument-relations]
/// - [Reflection][crate::Arg#reflection]
///
/// # Examples
///
/// ```rust
/// # use clap::{Arg, arg, ArgAction};
/// // Using the traditional builder pattern and setting each option manually
/// let cfg = Arg::new("config")
///       .short('c')
///       .long("config")
///       .action(ArgAction::Set)
///       .value_name("FILE")
///       .help("Provides a config file to myprog");
/// // Using a usage string (setting a similar argument to the one above)
/// let input = arg!(-i --input <FILE> "Provides an input file to the program");
/// ```
#[allow(missing_debug_implementations)]
#[derive(Default, Clone)]
pub struct Arg<'help> {
    pub(crate) id: Id,
    pub(crate) name: &'help str,
    pub(crate) help: Option<&'help str>,
    pub(crate) long_help: Option<&'help str>,
    pub(crate) action: Option<ArgAction>,
    pub(crate) value_parser: Option<super::ValueParser>,
    pub(crate) blacklist: Vec<Id>,
    pub(crate) settings: ArgFlags,
    pub(crate) overrides: Vec<Id>,
    pub(crate) groups: Vec<Id>,
    pub(crate) requires: Vec<(ArgPredicate<'help>, Id)>,
    pub(crate) r_ifs: Vec<(Id, &'help str)>,
    pub(crate) r_ifs_all: Vec<(Id, &'help str)>,
    pub(crate) r_unless: Vec<Id>,
    pub(crate) r_unless_all: Vec<Id>,
    pub(crate) short: Option<char>,
    pub(crate) long: Option<&'help str>,
    pub(crate) aliases: Vec<(&'help str, bool)>, // (name, visible)
    pub(crate) short_aliases: Vec<(char, bool)>, // (name, visible)
    pub(crate) disp_ord: Option<usize>,
    pub(crate) val_names: Vec<&'help str>,
    pub(crate) num_vals: Option<ValueRange>,
    pub(crate) val_delim: Option<char>,
    pub(crate) default_vals: Vec<&'help OsStr>,
    pub(crate) default_vals_ifs: Vec<(Id, ArgPredicate<'help>, Option<&'help OsStr>)>,
    pub(crate) default_missing_vals: Vec<&'help OsStr>,
    #[cfg(feature = "env")]
    pub(crate) env: Option<(&'help OsStr, Option<OsString>)>,
    pub(crate) terminator: Option<&'help str>,
    pub(crate) index: Option<usize>,
    pub(crate) help_heading: Option<Option<&'help str>>,
    pub(crate) value_hint: Option<ValueHint>,
}

/// # Basic API
impl<'help> Arg<'help> {
    /// Create a new [`Arg`] with a unique name.
    ///
    /// The name is used to check whether or not the argument was used at
    /// runtime, get values, set relationships with other args, etc..
    ///
    /// **NOTE:** In the case of arguments that take values (i.e. [`Arg::action(ArgAction::Set)`])
    /// and positional arguments (i.e. those without a preceding `-` or `--`) the name will also
    /// be displayed when the user prints the usage/help information of the program.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// Arg::new("config")
    /// # ;
    /// ```
    /// [`Arg::action(ArgAction::Set)`]: Arg::action()
    pub fn new<S: Into<&'help str>>(n: S) -> Self {
        Arg::default().id(n)
    }

    /// Set the identifier used for referencing this argument in the clap API.
    ///
    /// See [`Arg::new`] for more details.
    #[must_use]
    pub fn id<S: Into<&'help str>>(mut self, n: S) -> Self {
        let name = n.into();
        self.id = Id::from(name);
        self.name = name;
        self
    }

    /// Sets the short version of the argument without the preceding `-`.
    ///
    /// By default `V` and `h` are used by the auto-generated `version` and `help` arguments,
    /// respectively. You may use the uppercase `V` or lowercase `h` for your own arguments, in
    /// which case `clap` simply will not assign those to the auto-generated
    /// `version` or `help` arguments.
    ///
    /// # Examples
    ///
    /// When calling `short`, use a single valid UTF-8 character which will allow using the
    /// argument via a single hyphen (`-`) such as `-c`:
    ///
    /// ```rust
    /// # use clap::{Command, Arg,  ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("config")
    ///         .short('c')
    ///         .action(ArgAction::Set))
    ///     .get_matches_from(vec![
    ///         "prog", "-c", "file.toml"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("config").map(String::as_str), Some("file.toml"));
    /// ```
    #[inline]
    #[must_use]
    pub fn short(mut self, s: char) -> Self {
        assert!(s != '-', "short option name cannot be `-`");

        self.short = Some(s);
        self
    }

    /// Sets the long version of the argument without the preceding `--`.
    ///
    /// By default `version` and `help` are used by the auto-generated `version` and `help`
    /// arguments, respectively. You may use the word `version` or `help` for the long form of your
    /// own arguments, in which case `clap` simply will not assign those to the auto-generated
    /// `version` or `help` arguments.
    ///
    /// **NOTE:** Any leading `-` characters will be stripped
    ///
    /// # Examples
    ///
    /// To set `long` use a word containing valid UTF-8. If you supply a double leading
    /// `--` such as `--config` they will be stripped. Hyphens in the middle of the word, however,
    /// will *not* be stripped (i.e. `config-file` is allowed).
    ///
    /// Setting `long` allows using the argument via a double hyphen (`--`) such as `--config`
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .action(ArgAction::Set))
    ///     .get_matches_from(vec![
    ///         "prog", "--config", "file.toml"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("cfg").map(String::as_str), Some("file.toml"));
    /// ```
    #[inline]
    #[must_use]
    pub fn long(mut self, l: &'help str) -> Self {
        self.long = Some(l);
        self
    }

    /// Add an alias, which functions as a hidden long flag.
    ///
    /// This is more efficient, and easier than creating multiple hidden arguments as one only
    /// needs to check for the existence of this command, and not all variants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///             .arg(Arg::new("test")
    ///             .long("test")
    ///             .alias("alias")
    ///             .action(ArgAction::Set))
    ///        .get_matches_from(vec![
    ///             "prog", "--alias", "cool"
    ///         ]);
    /// assert_eq!(m.get_one::<String>("test").unwrap(), "cool");
    /// ```
    #[must_use]
    pub fn alias<S: Into<&'help str>>(mut self, name: S) -> Self {
        self.aliases.push((name.into(), false));
        self
    }

    /// Add an alias, which functions as a hidden short flag.
    ///
    /// This is more efficient, and easier than creating multiple hidden arguments as one only
    /// needs to check for the existence of this command, and not all variants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///             .arg(Arg::new("test")
    ///             .short('t')
    ///             .short_alias('e')
    ///             .action(ArgAction::Set))
    ///        .get_matches_from(vec![
    ///             "prog", "-e", "cool"
    ///         ]);
    /// assert_eq!(m.get_one::<String>("test").unwrap(), "cool");
    /// ```
    #[must_use]
    pub fn short_alias(mut self, name: char) -> Self {
        assert!(name != '-', "short alias name cannot be `-`");

        self.short_aliases.push((name, false));
        self
    }

    /// Add aliases, which function as hidden long flags.
    ///
    /// This is more efficient, and easier than creating multiple hidden subcommands as one only
    /// needs to check for the existence of this command, and not all variants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///             .arg(Arg::new("test")
    ///                     .long("test")
    ///                     .aliases(&["do-stuff", "do-tests", "tests"])
    ///                     .action(ArgAction::SetTrue)
    ///                     .help("the file to add")
    ///                     .required(false))
    ///             .get_matches_from(vec![
    ///                 "prog", "--do-tests"
    ///             ]);
    /// assert_eq!(*m.get_one::<bool>("test").expect("defaulted by clap"), true);
    /// ```
    #[must_use]
    pub fn aliases(mut self, names: &[&'help str]) -> Self {
        self.aliases.extend(names.iter().map(|&x| (x, false)));
        self
    }

    /// Add aliases, which functions as a hidden short flag.
    ///
    /// This is more efficient, and easier than creating multiple hidden subcommands as one only
    /// needs to check for the existence of this command, and not all variants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///             .arg(Arg::new("test")
    ///                     .short('t')
    ///                     .short_aliases(&['e', 's'])
    ///                     .action(ArgAction::SetTrue)
    ///                     .help("the file to add")
    ///                     .required(false))
    ///             .get_matches_from(vec![
    ///                 "prog", "-s"
    ///             ]);
    /// assert_eq!(*m.get_one::<bool>("test").expect("defaulted by clap"), true);
    /// ```
    #[must_use]
    pub fn short_aliases(mut self, names: &[char]) -> Self {
        for s in names {
            assert!(s != &'-', "short alias name cannot be `-`");
            self.short_aliases.push((*s, false));
        }
        self
    }

    /// Add an alias, which functions as a visible long flag.
    ///
    /// Like [`Arg::alias`], except that they are visible inside the help message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///             .arg(Arg::new("test")
    ///                 .visible_alias("something-awesome")
    ///                 .long("test")
    ///                 .action(ArgAction::Set))
    ///        .get_matches_from(vec![
    ///             "prog", "--something-awesome", "coffee"
    ///         ]);
    /// assert_eq!(m.get_one::<String>("test").unwrap(), "coffee");
    /// ```
    /// [`Command::alias`]: Arg::alias()
    #[must_use]
    pub fn visible_alias<S: Into<&'help str>>(mut self, name: S) -> Self {
        self.aliases.push((name.into(), true));
        self
    }

    /// Add an alias, which functions as a visible short flag.
    ///
    /// Like [`Arg::short_alias`], except that they are visible inside the help message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///             .arg(Arg::new("test")
    ///                 .long("test")
    ///                 .visible_short_alias('t')
    ///                 .action(ArgAction::Set))
    ///        .get_matches_from(vec![
    ///             "prog", "-t", "coffee"
    ///         ]);
    /// assert_eq!(m.get_one::<String>("test").unwrap(), "coffee");
    /// ```
    #[must_use]
    pub fn visible_short_alias(mut self, name: char) -> Self {
        assert!(name != '-', "short alias name cannot be `-`");

        self.short_aliases.push((name, true));
        self
    }

    /// Add aliases, which function as visible long flags.
    ///
    /// Like [`Arg::aliases`], except that they are visible inside the help message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///             .arg(Arg::new("test")
    ///                 .long("test")
    ///                 .action(ArgAction::SetTrue)
    ///                 .visible_aliases(&["something", "awesome", "cool"]))
    ///        .get_matches_from(vec![
    ///             "prog", "--awesome"
    ///         ]);
    /// assert_eq!(*m.get_one::<bool>("test").expect("defaulted by clap"), true);
    /// ```
    /// [`Command::aliases`]: Arg::aliases()
    #[must_use]
    pub fn visible_aliases(mut self, names: &[&'help str]) -> Self {
        self.aliases.extend(names.iter().map(|n| (*n, true)));
        self
    }

    /// Add aliases, which function as visible short flags.
    ///
    /// Like [`Arg::short_aliases`], except that they are visible inside the help message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///             .arg(Arg::new("test")
    ///                 .long("test")
    ///                 .action(ArgAction::SetTrue)
    ///                 .visible_short_aliases(&['t', 'e']))
    ///        .get_matches_from(vec![
    ///             "prog", "-t"
    ///         ]);
    /// assert_eq!(*m.get_one::<bool>("test").expect("defaulted by clap"), true);
    /// ```
    #[must_use]
    pub fn visible_short_aliases(mut self, names: &[char]) -> Self {
        for n in names {
            assert!(n != &'-', "short alias name cannot be `-`");
            self.short_aliases.push((*n, true));
        }
        self
    }

    /// Specifies the index of a positional argument **starting at** 1.
    ///
    /// **NOTE:** The index refers to position according to **other positional argument**. It does
    /// not define position in the argument list as a whole.
    ///
    /// **NOTE:** You can optionally leave off the `index` method, and the index will be
    /// assigned in order of evaluation. Utilizing the `index` method allows for setting
    /// indexes out of order
    ///
    /// **NOTE:** This is only meant to be used for positional arguments and shouldn't to be used
    /// with [`Arg::short`] or [`Arg::long`].
    ///
    /// **NOTE:** When utilized with [`Arg::num_args(1..)`], only the **last** positional argument
    /// may be defined as having a variable number of arguments (i.e. with the highest index)
    ///
    /// # Panics
    ///
    /// [`Command`] will [`panic!`] if indexes are skipped (such as defining `index(1)` and `index(3)`
    /// but not `index(2)`, or a positional argument is defined as multiple and is not the highest
    /// index
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// Arg::new("config")
    ///     .index(1)
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .index(1))
    ///     .arg(Arg::new("debug")
    ///         .long("debug")
    ///         .action(ArgAction::SetTrue))
    ///     .get_matches_from(vec![
    ///         "prog", "--debug", "fast"
    ///     ]);
    ///
    /// assert!(m.contains_id("mode"));
    /// assert_eq!(m.get_one::<String>("mode").unwrap(), "fast"); // notice index(1) means "first positional"
    ///                                                           // *not* first argument
    /// ```
    /// [`Arg::short`]: Arg::short()
    /// [`Arg::long`]: Arg::long()
    /// [`Arg::num_args(true)`]: Arg::num_args()
    /// [`panic!`]: https://doc.rust-lang.org/std/macro.panic!.html
    /// [`Command`]: crate::Command
    #[inline]
    #[must_use]
    pub fn index(mut self, idx: usize) -> Self {
        self.index = Some(idx);
        self
    }

    /// This arg is the last, or final, positional argument (i.e. has the highest
    /// index) and is *only* able to be accessed via the `--` syntax (i.e. `$ prog args --
    /// last_arg`).
    ///
    /// Even, if no other arguments are left to parse, if the user omits the `--` syntax
    /// they will receive an [`UnknownArgument`] error. Setting an argument to `.last(true)` also
    /// allows one to access this arg early using the `--` syntax. Accessing an arg early, even with
    /// the `--` syntax is otherwise not possible.
    ///
    /// **NOTE:** This will change the usage string to look like `$ prog [OPTIONS] [-- <ARG>]` if
    /// `ARG` is marked as `.last(true)`.
    ///
    /// **NOTE:** This setting will imply [`crate::Command::dont_collapse_args_in_usage`] because failing
    /// to set this can make the usage string very confusing.
    ///
    /// **NOTE**: This setting only applies to positional arguments, and has no effect on OPTIONS
    ///
    /// **NOTE:** Setting this requires [taking values][Arg::num_args]
    ///
    /// **CAUTION:** Using this setting *and* having child subcommands is not
    /// recommended with the exception of *also* using
    /// [`crate::Command::args_conflicts_with_subcommands`]
    /// (or [`crate::Command::subcommand_negates_reqs`] if the argument marked `Last` is also
    /// marked [`Arg::required`])
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Arg, ArgAction};
    /// Arg::new("args")
    ///     .action(ArgAction::Set)
    ///     .last(true)
    /// # ;
    /// ```
    ///
    /// Setting `last` ensures the arg has the highest [index] of all positional args
    /// and requires that the `--` syntax be used to access it early.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("first"))
    ///     .arg(Arg::new("second"))
    ///     .arg(Arg::new("third")
    ///         .action(ArgAction::Set)
    ///         .last(true))
    ///     .try_get_matches_from(vec![
    ///         "prog", "one", "--", "three"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// let m = res.unwrap();
    /// assert_eq!(m.get_one::<String>("third").unwrap(), "three");
    /// assert_eq!(m.get_one::<String>("second"), None);
    /// ```
    ///
    /// Even if the positional argument marked `Last` is the only argument left to parse,
    /// failing to use the `--` syntax results in an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("first"))
    ///     .arg(Arg::new("second"))
    ///     .arg(Arg::new("third")
    ///         .action(ArgAction::Set)
    ///         .last(true))
    ///     .try_get_matches_from(vec![
    ///         "prog", "one", "two", "three"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::UnknownArgument);
    /// ```
    /// [index]: Arg::index()
    /// [`UnknownArgument`]: crate::error::ErrorKind::UnknownArgument
    #[inline]
    #[must_use]
    pub fn last(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::Last)
        } else {
            self.unset_setting(ArgSettings::Last)
        }
    }

    /// Specifies that the argument must be present.
    ///
    /// Required by default means it is required, when no other conflicting rules or overrides have
    /// been evaluated. Conflicting rules take precedence over being required.
    ///
    /// **Pro tip:** Flags (i.e. not positional, or arguments that take values) shouldn't be
    /// required by default. This is because if a flag were to be required, it should simply be
    /// implied. No additional information is required from user. Flags by their very nature are
    /// simply boolean on/off switches. The only time a user *should* be required to use a flag
    /// is if the operation is destructive in nature, and the user is essentially proving to you,
    /// "Yes, I know what I'm doing."
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required(true)
    /// # ;
    /// ```
    ///
    /// Setting required requires that the argument be used at runtime.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required(true)
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "file.conf",
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// Setting required and then *not* supplying that argument at runtime is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required(true)
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    #[inline]
    #[must_use]
    pub fn required(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::Required)
        } else {
            self.unset_setting(ArgSettings::Required)
        }
    }

    /// Sets an argument that is required when this one is present
    ///
    /// i.e. when using this argument, the following argument *must* be present.
    ///
    /// **NOTE:** [Conflicting] rules and [override] rules take precedence over being required
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .requires("input")
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::requires(name)`] requires that the argument be used at runtime if the
    /// defining argument is used. If the defining argument isn't used, the other argument isn't
    /// required
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .requires("input")
    ///         .long("config"))
    ///     .arg(Arg::new("input"))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use cfg, so input wasn't required
    /// ```
    ///
    /// Setting [`Arg::requires(name)`] and *not* supplying that argument is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .requires("input")
    ///         .long("config"))
    ///     .arg(Arg::new("input"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "file.conf"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::requires(name)`]: Arg::requires()
    /// [Conflicting]: Arg::conflicts_with()
    /// [override]: Arg::overrides_with()
    #[must_use]
    pub fn requires<T: Key>(mut self, arg_id: T) -> Self {
        self.requires.push((ArgPredicate::IsPresent, arg_id.into()));
        self
    }

    /// This argument must be passed alone; it conflicts with all other arguments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .exclusive(true)
    /// # ;
    /// ```
    ///
    /// Setting an exclusive argument and having any other arguments present at runtime
    /// is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("exclusive")
    ///         .action(ArgAction::Set)
    ///         .exclusive(true)
    ///         .long("exclusive"))
    ///     .arg(Arg::new("debug")
    ///         .long("debug"))
    ///     .arg(Arg::new("input"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--exclusive", "file.conf", "file.txt"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::ArgumentConflict);
    /// ```
    #[inline]
    #[must_use]
    pub fn exclusive(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::Exclusive)
        } else {
            self.unset_setting(ArgSettings::Exclusive)
        }
    }

    /// Specifies that an argument can be matched to all child [`Subcommand`]s.
    ///
    /// **NOTE:** Global arguments *only* propagate down, **not** up (to parent commands), however
    /// their values once a user uses them will be propagated back up to parents. In effect, this
    /// means one should *define* all global arguments at the top level, however it doesn't matter
    /// where the user *uses* the global argument.
    ///
    /// # Examples
    ///
    /// Assume an application with two subcommands, and you'd like to define a
    /// `--verbose` flag that can be called on any of the subcommands and parent, but you don't
    /// want to clutter the source with three duplicate [`Arg`] definitions.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("verb")
    ///         .long("verbose")
    ///         .short('v')
    ///         .action(ArgAction::SetTrue)
    ///         .global(true))
    ///     .subcommand(Command::new("test"))
    ///     .subcommand(Command::new("do-stuff"))
    ///     .get_matches_from(vec![
    ///         "prog", "do-stuff", "--verbose"
    ///     ]);
    ///
    /// assert_eq!(m.subcommand_name(), Some("do-stuff"));
    /// let sub_m = m.subcommand_matches("do-stuff").unwrap();
    /// assert_eq!(*sub_m.get_one::<bool>("verb").expect("defaulted by clap"), true);
    /// ```
    ///
    /// [`Subcommand`]: crate::Subcommand
    #[inline]
    #[must_use]
    pub fn global(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::Global)
        } else {
            self.unset_setting(ArgSettings::Global)
        }
    }

    #[inline]
    pub(crate) fn is_set(&self, s: ArgSettings) -> bool {
        self.settings.is_set(s)
    }

    #[inline]
    #[must_use]
    pub(crate) fn setting<F>(mut self, setting: F) -> Self
    where
        F: Into<ArgFlags>,
    {
        self.settings.insert(setting.into());
        self
    }

    #[inline]
    #[must_use]
    pub(crate) fn unset_setting<F>(mut self, setting: F) -> Self
    where
        F: Into<ArgFlags>,
    {
        self.settings.remove(setting.into());
        self
    }
}

/// # Value Handling
impl<'help> Arg<'help> {
    /// Specify how to react to an argument when parsing it.
    ///
    /// [ArgAction][crate::ArgAction] controls things like
    /// - Overwriting previous values with new ones
    /// - Appending new values to all previous ones
    /// - Counting how many times a flag occurs
    ///
    /// The default action is `ArgAction::Set`
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Command;
    /// # use clap::Arg;
    /// let cmd = Command::new("mycmd")
    ///     .arg(
    ///         Arg::new("flag")
    ///             .long("flag")
    ///             .action(clap::ArgAction::Append)
    ///     );
    ///
    /// let matches = cmd.try_get_matches_from(["mycmd", "--flag", "value"]).unwrap();
    /// assert!(matches.contains_id("flag"));
    /// assert_eq!(
    ///     matches.get_many::<String>("flag").unwrap_or_default().map(|v| v.as_str()).collect::<Vec<_>>(),
    ///     vec!["value"]
    /// );
    /// ```
    #[inline]
    #[must_use]
    pub fn action(mut self, action: ArgAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Specify the type of the argument.
    ///
    /// This allows parsing and validating a value before storing it into
    /// [`ArgMatches`][crate::ArgMatches].
    ///
    /// The default value is [`ValueParser::string`][crate::builder::ValueParser::string].
    ///
    /// See also
    /// - [`value_parser!`][crate::value_parser!] for auto-selecting a value parser for a given type
    ///   - [`BoolishValueParser`][crate::builder::BoolishValueParser], and [`FalseyValueParser`][crate::builder::FalseyValueParser] for alternative `bool` implementations
    ///   - [`NonEmptyStringValueParser`][crate::builder::NonEmptyStringValueParser] for basic validation for strings
    /// - [`RangedI64ValueParser`][crate::builder::RangedI64ValueParser] and [`RangedU64ValueParser`][crate::builder::RangedU64ValueParser] for numeric ranges
    /// - [`EnumValueParser`][crate::builder::EnumValueParser] and  [`PossibleValuesParser`][crate::builder::PossibleValuesParser] for static enumerated values
    /// - or any other [`TypedValueParser`][crate::builder::TypedValueParser] implementation
    ///
    /// ```rust
    /// # use clap::ArgAction;
    /// let mut cmd = clap::Command::new("raw")
    ///     .arg(
    ///         clap::Arg::new("color")
    ///             .long("color")
    ///             .value_parser(["always", "auto", "never"])
    ///             .default_value("auto")
    ///     )
    ///     .arg(
    ///         clap::Arg::new("hostname")
    ///             .long("hostname")
    ///             .value_parser(clap::builder::NonEmptyStringValueParser::new())
    ///             .action(ArgAction::Set)
    ///             .required(true)
    ///     )
    ///     .arg(
    ///         clap::Arg::new("port")
    ///             .long("port")
    ///             .value_parser(clap::value_parser!(u16).range(3000..))
    ///             .action(ArgAction::Set)
    ///             .required(true)
    ///     );
    ///
    /// let m = cmd.try_get_matches_from_mut(
    ///     ["cmd", "--hostname", "rust-lang.org", "--port", "3001"]
    /// ).unwrap();
    ///
    /// let color: &String = m.get_one("color")
    ///     .expect("default");
    /// assert_eq!(color, "auto");
    ///
    /// let hostname: &String = m.get_one("hostname")
    ///     .expect("required");
    /// assert_eq!(hostname, "rust-lang.org");
    ///
    /// let port: u16 = *m.get_one("port")
    ///     .expect("required");
    /// assert_eq!(port, 3001);
    /// ```
    pub fn value_parser(mut self, parser: impl Into<super::ValueParser>) -> Self {
        self.value_parser = Some(parser.into());
        self
    }

    /// Specifies the number of arguments parsed per occurrence
    ///
    /// For example, if you had a `-f <file>` argument where you wanted exactly 3 'files' you would
    /// set `.num_args(3)`, and this argument wouldn't be satisfied unless the user
    /// provided 3 and only 3 values.
    ///
    /// Users may specify values for arguments in any of the following methods
    ///
    /// - Using a space such as `-o value` or `--option value`
    /// - Using an equals and no space such as `-o=value` or `--option=value`
    /// - Use a short and no space such as `-ovalue`
    ///
    /// **WARNING:**
    ///
    /// Setting a variable number of values (e.g. `1..=10`) for an argument without
    /// other details can be dangerous in some circumstances. Because multiple values are
    /// allowed, `--option val1 val2 val3` is perfectly valid. Be careful when designing a CLI
    /// where **positional arguments** or **subcommands** are *also* expected as `clap` will continue
    /// parsing *values* until one of the following happens:
    ///
    /// - It reaches the maximum number of values
    /// - It reaches a specific number of values
    /// - It finds another flag or option (i.e. something that starts with a `-`)
    /// - It reaches the [`Arg::value_terminator`] if set
    ///
    /// Alternatively,
    /// - Use a delimiter between values with [Arg::value_delimiter]
    /// - Require a flag occurrence per value with [`ArgAction::Append`]
    /// - Require positional arguments to appear after `--` with [`Arg::last`]
    ///
    /// # Examples
    ///
    /// Option:
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .long("mode")
    ///         .num_args(1))
    ///     .get_matches_from(vec![
    ///         "prog", "--mode", "fast"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("mode").unwrap(), "fast");
    /// ```
    ///
    /// Flag/option hybrid (see also [default_missing_value][Arg::default_missing_value])
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let cmd = Command::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .long("mode")
    ///         .default_missing_value("slow")
    ///         .default_value("plaid")
    ///         .num_args(0..=1));
    ///
    /// let m = cmd.clone()
    ///     .get_matches_from(vec![
    ///         "prog", "--mode", "fast"
    ///     ]);
    /// assert_eq!(m.get_one::<String>("mode").unwrap(), "fast");
    ///
    /// let m = cmd.clone()
    ///     .get_matches_from(vec![
    ///         "prog", "--mode",
    ///     ]);
    /// assert_eq!(m.get_one::<String>("mode").unwrap(), "slow");
    ///
    /// let m = cmd.clone()
    ///     .get_matches_from(vec![
    ///         "prog",
    ///     ]);
    /// assert_eq!(m.get_one::<String>("mode").unwrap(), "plaid");
    /// ```
    ///
    /// Tuples
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let cmd = Command::new("prog")
    ///     .arg(Arg::new("file")
    ///         .action(ArgAction::Set)
    ///         .num_args(2)
    ///         .short('F'));
    ///
    /// let m = cmd.clone()
    ///     .get_matches_from(vec![
    ///         "prog", "-F", "in-file", "out-file"
    ///     ]);
    /// assert_eq!(
    ///     m.get_many::<String>("file").unwrap_or_default().map(|v| v.as_str()).collect::<Vec<_>>(),
    ///     vec!["in-file", "out-file"]
    /// );
    ///
    /// let res = cmd.clone()
    ///     .try_get_matches_from(vec![
    ///         "prog", "-F", "file1"
    ///     ]);
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::WrongNumberOfValues);
    /// ```
    ///
    /// A common mistake is to define an option which allows multiple values and a positional
    /// argument.
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let cmd = Command::new("prog")
    ///     .arg(Arg::new("file")
    ///         .action(ArgAction::Set)
    ///         .num_args(0..)
    ///         .short('F'))
    ///     .arg(Arg::new("word"));
    ///
    /// let m = cmd.clone().get_matches_from(vec![
    ///     "prog", "-F", "file1", "file2", "file3", "word"
    /// ]);
    /// let files: Vec<_> = m.get_many::<String>("file").unwrap().collect();
    /// assert_eq!(files, ["file1", "file2", "file3", "word"]); // wait...what?!
    /// assert!(!m.contains_id("word")); // but we clearly used word!
    ///
    /// // but this works
    /// let m = cmd.clone().get_matches_from(vec![
    ///     "prog", "word", "-F", "file1", "file2", "file3",
    /// ]);
    /// let files: Vec<_> = m.get_many::<String>("file").unwrap().collect();
    /// assert_eq!(files, ["file1", "file2", "file3"]);
    /// assert_eq!(m.get_one::<String>("word").unwrap(), "word");
    /// ```
    /// The problem is `clap` doesn't know when to stop parsing values for "file".
    ///
    /// A solution for the example above is to limit how many values with a maximum, or specific
    /// number, or to say [`ArgAction::Append`] is ok, but multiple values are not.
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("file")
    ///         .action(ArgAction::Append)
    ///         .short('F'))
    ///     .arg(Arg::new("word"))
    ///     .get_matches_from(vec![
    ///         "prog", "-F", "file1", "-F", "file2", "-F", "file3", "word"
    ///     ]);
    ///
    /// let files: Vec<_> = m.get_many::<String>("file").unwrap().collect();
    /// assert_eq!(files, ["file1", "file2", "file3"]);
    /// assert_eq!(m.get_one::<String>("word").unwrap(), "word");
    /// ```
    #[inline]
    #[must_use]
    pub fn num_args(mut self, qty: impl Into<ValueRange>) -> Self {
        let qty = qty.into();
        self.num_vals = Some(qty);
        self
    }

    /// Placeholder for the argument's value in the help message / usage.
    ///
    /// This name is cosmetic only; the name is **not** used to access arguments.
    /// This setting can be very helpful when describing the type of input the user should be
    /// using, such as `FILE`, `INTERFACE`, etc. Although not required, it's somewhat convention to
    /// use all capital letters for the value name.
    ///
    /// **NOTE:** implicitly sets [`Arg::action(ArgAction::Set)`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// Arg::new("cfg")
    ///     .long("config")
    ///     .value_name("FILE")
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("config")
    ///         .long("config")
    ///         .value_name("FILE")
    ///         .help("Some help text"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    /// Running the above program produces the following output
    ///
    /// ```text
    /// valnames
    ///
    /// USAGE:
    ///    valnames [OPTIONS]
    ///
    /// OPTIONS:
    ///     --config <FILE>     Some help text
    ///     -h, --help          Print help information
    ///     -V, --version       Print version information
    /// ```
    /// [positional]: Arg::index()
    /// [`Arg::action(ArgAction::Set)`]: Arg::action()
    #[inline]
    #[must_use]
    pub fn value_name(self, name: &'help str) -> Self {
        self.value_names(&[name])
    }

    /// Placeholders for the argument's values in the help message / usage.
    ///
    /// These names are cosmetic only, used for help and usage strings only. The names are **not**
    /// used to access arguments. The values of the arguments are accessed in numeric order (i.e.
    /// if you specify two names `one` and `two` `one` will be the first matched value, `two` will
    /// be the second).
    ///
    /// This setting can be very helpful when describing the type of input the user should be
    /// using, such as `FILE`, `INTERFACE`, etc. Although not required, it's somewhat convention to
    /// use all capital letters for the value name.
    ///
    /// **Pro Tip:** It may help to use [`Arg::next_line_help(true)`] if there are long, or
    /// multiple value names in order to not throw off the help text alignment of all options.
    ///
    /// **NOTE:** implicitly sets [`Arg::action(ArgAction::Set)`] and [`Arg::num_args(1..)`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// Arg::new("speed")
    ///     .short('s')
    ///     .value_names(&["fast", "slow"]);
    /// ```
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("io")
    ///         .long("io-files")
    ///         .value_names(&["INFILE", "OUTFILE"]))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// Running the above program produces the following output
    ///
    /// ```text
    /// valnames
    ///
    /// USAGE:
    ///    valnames [OPTIONS]
    ///
    /// OPTIONS:
    ///     -h, --help                       Print help information
    ///     --io-files <INFILE> <OUTFILE>    Some help text
    ///     -V, --version                    Print version information
    /// ```
    /// [`Arg::next_line_help(true)`]: Arg::next_line_help()
    /// [`Arg::num_args`]: Arg::num_args()
    /// [`Arg::action(ArgAction::Set)`]: Arg::action()
    /// [`Arg::num_args(1..)`]: Arg::num_args()
    #[must_use]
    pub fn value_names(mut self, names: &[&'help str]) -> Self {
        self.val_names = names.to_vec();
        self
    }

    /// Provide the shell a hint about how to complete this argument.
    ///
    /// See [`ValueHint`][crate::ValueHint] for more information.
    ///
    /// **NOTE:** implicitly sets [`Arg::action(ArgAction::Set)`].
    ///
    /// For example, to take a username as argument:
    ///
    /// ```
    /// # use clap::{Arg, ValueHint};
    /// Arg::new("user")
    ///     .short('u')
    ///     .long("user")
    ///     .value_hint(ValueHint::Username);
    /// ```
    ///
    /// To take a full command line and its arguments (for example, when writing a command wrapper):
    ///
    /// ```
    /// # use clap::{Command, Arg, ValueHint, ArgAction};
    /// Command::new("prog")
    ///     .trailing_var_arg(true)
    ///     .arg(
    ///         Arg::new("command")
    ///             .action(ArgAction::Set)
    ///             .num_args(1..)
    ///             .value_hint(ValueHint::CommandWithArguments)
    ///     );
    /// ```
    #[must_use]
    pub fn value_hint(mut self, value_hint: ValueHint) -> Self {
        self.value_hint = Some(value_hint);
        self
    }

    /// Match values against [`PossibleValuesParser`][crate::builder::PossibleValuesParser] without matching case.
    ///
    /// When other arguments are conditionally required based on the
    /// value of a case-insensitive argument, the equality check done
    /// by [`Arg::required_if_eq`], [`Arg::required_if_eq_any`], or
    /// [`Arg::required_if_eq_all`] is case-insensitive.
    ///
    ///
    /// **NOTE:** Setting this requires [taking values][Arg::num_args]
    ///
    /// **NOTE:** To do unicode case folding, enable the `unicode` feature flag.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("pv")
    ///     .arg(Arg::new("option")
    ///         .long("option")
    ///         .action(ArgAction::Set)
    ///         .ignore_case(true)
    ///         .value_parser(["test123"]))
    ///     .get_matches_from(vec![
    ///         "pv", "--option", "TeSt123",
    ///     ]);
    ///
    /// assert!(m.get_one::<String>("option").unwrap().eq_ignore_ascii_case("test123"));
    /// ```
    ///
    /// This setting also works when multiple values can be defined:
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("pv")
    ///     .arg(Arg::new("option")
    ///         .short('o')
    ///         .long("option")
    ///         .action(ArgAction::Set)
    ///         .ignore_case(true)
    ///         .num_args(1..)
    ///         .value_parser(["test123", "test321"]))
    ///     .get_matches_from(vec![
    ///         "pv", "--option", "TeSt123", "teST123", "tESt321"
    ///     ]);
    ///
    /// let matched_vals = m.get_many::<String>("option").unwrap().collect::<Vec<_>>();
    /// assert_eq!(&*matched_vals, &["TeSt123", "teST123", "tESt321"]);
    /// ```
    #[inline]
    #[must_use]
    pub fn ignore_case(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::IgnoreCase)
        } else {
            self.unset_setting(ArgSettings::IgnoreCase)
        }
    }

    /// Allows values which start with a leading hyphen (`-`)
    ///
    /// **NOTE:** Setting this requires [taking values][Arg::num_args]
    ///
    /// **WARNING**: Take caution when using this setting combined with
    /// [`Arg::num_args`], as this becomes ambiguous `$ prog --arg -- -- val`. All
    /// three `--, --, val` will be values when the user may have thought the second `--` would
    /// constitute the normal, "Only positional args follow" idiom.
    ///
    /// **WARNING**: When building your CLIs, consider the effects of allowing leading hyphens and
    /// the user passing in a value that matches a valid short. For example, `prog -opt -F` where
    /// `-F` is supposed to be a value, yet `-F` is *also* a valid short for another arg.
    /// Care should be taken when designing these args. This is compounded by the ability to "stack"
    /// short args. I.e. if `-val` is supposed to be a value, but `-v`, `-a`, and `-l` are all valid
    /// shorts.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("pat")
    ///         .action(ArgAction::Set)
    ///         .allow_hyphen_values(true)
    ///         .long("pattern"))
    ///     .get_matches_from(vec![
    ///         "prog", "--pattern", "-file"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("pat").unwrap(), "-file");
    /// ```
    ///
    /// Not setting `Arg::allow_hyphen_values(true)` and supplying a value which starts with a
    /// hyphen is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("pat")
    ///         .action(ArgAction::Set)
    ///         .long("pattern"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--pattern", "-file"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::UnknownArgument);
    /// ```
    /// [`Arg::num_args(1)`]: Arg::num_args()
    #[inline]
    #[must_use]
    pub fn allow_hyphen_values(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::AllowHyphenValues)
        } else {
            self.unset_setting(ArgSettings::AllowHyphenValues)
        }
    }

    /// Requires that options use the `--option=val` syntax
    ///
    /// i.e. an equals between the option and associated value.
    ///
    /// **NOTE:** Setting this requires [taking values][Arg::num_args]
    ///
    /// # Examples
    ///
    /// Setting `require_equals` requires that the option have an equals sign between
    /// it and the associated value.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .require_equals(true)
    ///         .long("config"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config=file.conf"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// Setting `require_equals` and *not* supplying the equals will cause an
    /// error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .require_equals(true)
    ///         .long("config"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "file.conf"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::NoEquals);
    /// ```
    #[inline]
    #[must_use]
    pub fn require_equals(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::RequireEquals)
        } else {
            self.unset_setting(ArgSettings::RequireEquals)
        }
    }

    /// Allow grouping of multiple values via a delimiter.
    ///
    /// i.e. should `--option=val1,val2,val3` be parsed as three values (`val1`, `val2`,
    /// and `val3`) or as a single value (`val1,val2,val3`). Defaults to using `,` (comma) as the
    /// value delimiter for all arguments that accept values (options and positional arguments)
    ///
    /// **NOTE:** implicitly sets [`Arg::action(ArgAction::Set)`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("config")
    ///         .short('c')
    ///         .long("config")
    ///         .value_delimiter(','))
    ///     .get_matches_from(vec![
    ///         "prog", "--config=val1,val2,val3"
    ///     ]);
    ///
    /// assert_eq!(m.get_many::<String>("config").unwrap().collect::<Vec<_>>(), ["val1", "val2", "val3"])
    /// ```
    /// [`Arg::value_delimiter(',')`]: Arg::value_delimiter()
    /// [`Arg::action(ArgAction::Set)`]: Arg::action()
    #[inline]
    #[must_use]
    pub fn value_delimiter(mut self, d: impl Into<Option<char>>) -> Self {
        self.val_delim = d.into();
        self
    }

    /// Sentinel to **stop** parsing multiple values of a given argument.
    ///
    /// By default when
    /// one sets [`num_args(1..)`] on an argument, clap will continue parsing values for that
    /// argument until it reaches another valid argument, or one of the other more specific settings
    /// for multiple values is used (such as [`num_args`]).
    ///
    /// **NOTE:** This setting only applies to [options] and [positional arguments]
    ///
    /// **NOTE:** When the terminator is passed in on the command line, it is **not** stored as one
    /// of the values
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// Arg::new("vals")
    ///     .action(ArgAction::Set)
    ///     .num_args(1..)
    ///     .value_terminator(";")
    /// # ;
    /// ```
    ///
    /// The following example uses two arguments, a sequence of commands, and the location in which
    /// to perform them
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("cmds")
    ///         .action(ArgAction::Set)
    ///         .num_args(1..)
    ///         .allow_hyphen_values(true)
    ///         .value_terminator(";"))
    ///     .arg(Arg::new("location"))
    ///     .get_matches_from(vec![
    ///         "prog", "find", "-type", "f", "-name", "special", ";", "/home/clap"
    ///     ]);
    /// let cmds: Vec<_> = m.get_many::<String>("cmds").unwrap().collect();
    /// assert_eq!(&cmds, &["find", "-type", "f", "-name", "special"]);
    /// assert_eq!(m.get_one::<String>("location").unwrap(), "/home/clap");
    /// ```
    /// [options]: Arg::action
    /// [positional arguments]: Arg::index()
    /// [`num_args(1..)`]: Arg::num_args()
    /// [`num_args`]: Arg::num_args()
    #[inline]
    #[must_use]
    pub fn value_terminator(mut self, term: &'help str) -> Self {
        self.terminator = Some(term);
        self
    }

    /// Consume all following arguments.
    ///
    /// Do not be parse them individually, but rather pass them in entirety.
    ///
    /// It is worth noting that setting this requires all values to come after a `--` to indicate
    /// they should all be captured. For example:
    ///
    /// ```text
    /// --foo something -- -v -v -v -b -b -b --baz -q -u -x
    /// ```
    ///
    /// Will result in everything after `--` to be considered one raw argument. This behavior
    /// may not be exactly what you are expecting and using [`crate::Command::trailing_var_arg`]
    /// may be more appropriate.
    ///
    /// **NOTE:** Implicitly sets [`Arg::action(ArgAction::Set)`] [`Arg::num_args(1..)`],
    /// [`Arg::allow_hyphen_values(true)`], and [`Arg::last(true)`] when set to `true`.
    ///
    /// [`Arg::action(ArgAction::Set)`]: Arg::action()
    /// [`Arg::num_args(1..)`]: Arg::num_args()
    /// [`Arg::allow_hyphen_values(true)`]: Arg::allow_hyphen_values()
    /// [`Arg::last(true)`]: Arg::last()
    #[inline]
    #[must_use]
    pub fn raw(mut self, yes: bool) -> Self {
        if yes {
            self.num_vals.get_or_insert_with(|| (1..).into());
        }
        self.allow_hyphen_values(yes).last(yes)
    }

    /// Value for the argument when not present.
    ///
    /// **NOTE:** If the user *does not* use this argument at runtime [`ArgMatches::contains_id`] will
    /// still return `true`. If you wish to determine whether the argument was used at runtime or
    /// not, consider [`ArgMatches::value_source`][crate::ArgMatches::value_source].
    ///
    /// **NOTE:** This setting is perfectly compatible with [`Arg::default_value_if`] but slightly
    /// different. `Arg::default_value` *only* takes effect when the user has not provided this arg
    /// at runtime. `Arg::default_value_if` however only takes effect when the user has not provided
    /// a value at runtime **and** these other conditions are met as well. If you have set
    /// `Arg::default_value` and `Arg::default_value_if`, and the user **did not** provide this arg
    /// at runtime, nor were the conditions met for `Arg::default_value_if`, the `Arg::default_value`
    /// will be applied.
    ///
    /// **NOTE:** This implicitly sets [`Arg::action(ArgAction::Set)`].
    ///
    /// # Examples
    ///
    /// First we use the default value without providing any value at runtime.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, parser::ValueSource};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .long("myopt")
    ///         .default_value("myval"))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("opt").unwrap(), "myval");
    /// assert!(m.contains_id("opt"));
    /// assert_eq!(m.value_source("opt"), Some(ValueSource::DefaultValue));
    /// ```
    ///
    /// Next we provide a value at runtime to override the default.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, parser::ValueSource};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .long("myopt")
    ///         .default_value("myval"))
    ///     .get_matches_from(vec![
    ///         "prog", "--myopt=non_default"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("opt").unwrap(), "non_default");
    /// assert!(m.contains_id("opt"));
    /// assert_eq!(m.value_source("opt"), Some(ValueSource::CommandLine));
    /// ```
    /// [`Arg::action(ArgAction::Set)`]: Arg::action()
    /// [`ArgMatches::contains_id`]: crate::ArgMatches::contains_id()
    /// [`Arg::default_value_if`]: Arg::default_value_if()
    #[inline]
    #[must_use]
    pub fn default_value(self, val: &'help str) -> Self {
        self.default_values_os(&[OsStr::new(val)])
    }

    /// Value for the argument when not present.
    ///
    /// See [`Arg::default_value`].
    ///
    /// [`Arg::default_value`]: Arg::default_value()
    /// [`OsStr`]: std::ffi::OsStr
    #[inline]
    #[must_use]
    pub fn default_value_os(self, val: &'help OsStr) -> Self {
        self.default_values_os(&[val])
    }

    /// Value for the argument when not present.
    ///
    /// See [`Arg::default_value`].
    ///
    /// [`Arg::default_value`]: Arg::default_value()
    #[inline]
    #[must_use]
    pub fn default_values(self, vals: &[&'help str]) -> Self {
        let vals_vec: Vec<_> = vals.iter().map(|val| OsStr::new(*val)).collect();
        self.default_values_os(&vals_vec[..])
    }

    /// Value for the argument when not present.
    ///
    /// See [`Arg::default_values`].
    ///
    /// [`Arg::default_values`]: Arg::default_values()
    /// [`OsStr`]: std::ffi::OsStr
    #[inline]
    #[must_use]
    pub fn default_values_os(mut self, vals: &[&'help OsStr]) -> Self {
        self.default_vals = vals.to_vec();
        self
    }

    /// Value for the argument when the flag is present but no value is specified.
    ///
    /// This configuration option is often used to give the user a shortcut and allow them to
    /// efficiently specify an option argument without requiring an explicitly value. The `--color`
    /// argument is a common example. By, supplying an default, such as `default_missing_value("always")`,
    /// the user can quickly just add `--color` to the command line to produce the desired color output.
    ///
    /// **NOTE:** using this configuration option requires the use of the
    /// [`.num_args(0..N)`][Arg::num_args] and the
    /// [`.require_equals(true)`][Arg::require_equals] configuration option. These are required in
    /// order to unambiguously determine what, if any, value was supplied for the argument.
    ///
    /// # Examples
    ///
    /// For POSIX style `--color`:
    /// ```rust
    /// # use clap::{Command, Arg, parser::ValueSource};
    /// fn cli() -> Command<'static> {
    ///     Command::new("prog")
    ///         .arg(Arg::new("color").long("color")
    ///             .value_name("WHEN")
    ///             .value_parser(["always", "auto", "never"])
    ///             .default_value("auto")
    ///             .num_args(0..=1)
    ///             .require_equals(true)
    ///             .default_missing_value("always")
    ///             .help("Specify WHEN to colorize output.")
    ///         )
    /// }
    ///
    /// // first, we'll provide no arguments
    /// let m  = cli().get_matches_from(vec![
    ///         "prog"
    ///     ]);
    /// assert_eq!(m.get_one::<String>("color").unwrap(), "auto");
    /// assert_eq!(m.value_source("color"), Some(ValueSource::DefaultValue));
    ///
    /// // next, we'll provide a runtime value to override the default (as usually done).
    /// let m  = cli().get_matches_from(vec![
    ///         "prog", "--color=never"
    ///     ]);
    /// assert_eq!(m.get_one::<String>("color").unwrap(), "never");
    /// assert_eq!(m.value_source("color"), Some(ValueSource::CommandLine));
    ///
    /// // finally, we will use the shortcut and only provide the argument without a value.
    /// let m  = cli().get_matches_from(vec![
    ///         "prog", "--color"
    ///     ]);
    /// assert_eq!(m.get_one::<String>("color").unwrap(), "always");
    /// assert_eq!(m.value_source("color"), Some(ValueSource::CommandLine));
    /// ```
    ///
    /// For bool literals:
    /// ```rust
    /// # use clap::{Command, Arg, parser::ValueSource, value_parser};
    /// fn cli() -> Command<'static> {
    ///     Command::new("prog")
    ///         .arg(Arg::new("create").long("create")
    ///             .value_name("BOOL")
    ///             .value_parser(value_parser!(bool))
    ///             .num_args(0..=1)
    ///             .require_equals(true)
    ///             .default_missing_value("true")
    ///         )
    /// }
    ///
    /// // first, we'll provide no arguments
    /// let m  = cli().get_matches_from(vec![
    ///         "prog"
    ///     ]);
    /// assert_eq!(m.get_one::<bool>("create").copied(), None);
    ///
    /// // next, we'll provide a runtime value to override the default (as usually done).
    /// let m  = cli().get_matches_from(vec![
    ///         "prog", "--create=false"
    ///     ]);
    /// assert_eq!(m.get_one::<bool>("create").copied(), Some(false));
    /// assert_eq!(m.value_source("create"), Some(ValueSource::CommandLine));
    ///
    /// // finally, we will use the shortcut and only provide the argument without a value.
    /// let m  = cli().get_matches_from(vec![
    ///         "prog", "--create"
    ///     ]);
    /// assert_eq!(m.get_one::<bool>("create").copied(), Some(true));
    /// assert_eq!(m.value_source("create"), Some(ValueSource::CommandLine));
    /// ```
    ///
    /// [`Arg::action(ArgAction::Set)`]: Arg::action()
    /// [`Arg::default_value`]: Arg::default_value()
    #[inline]
    #[must_use]
    pub fn default_missing_value(self, val: &'help str) -> Self {
        self.default_missing_values_os(&[OsStr::new(val)])
    }

    /// Value for the argument when the flag is present but no value is specified.
    ///
    /// See [`Arg::default_missing_value`].
    ///
    /// [`Arg::default_missing_value`]: Arg::default_missing_value()
    /// [`OsStr`]: std::ffi::OsStr
    #[inline]
    #[must_use]
    pub fn default_missing_value_os(self, val: &'help OsStr) -> Self {
        self.default_missing_values_os(&[val])
    }

    /// Value for the argument when the flag is present but no value is specified.
    ///
    /// See [`Arg::default_missing_value`].
    ///
    /// [`Arg::default_missing_value`]: Arg::default_missing_value()
    #[inline]
    #[must_use]
    pub fn default_missing_values(self, vals: &[&'help str]) -> Self {
        let vals_vec: Vec<_> = vals.iter().map(|val| OsStr::new(*val)).collect();
        self.default_missing_values_os(&vals_vec[..])
    }

    /// Value for the argument when the flag is present but no value is specified.
    ///
    /// See [`Arg::default_missing_values`].
    ///
    /// [`Arg::default_missing_values`]: Arg::default_missing_values()
    /// [`OsStr`]: std::ffi::OsStr
    #[inline]
    #[must_use]
    pub fn default_missing_values_os(mut self, vals: &[&'help OsStr]) -> Self {
        self.default_missing_vals = vals.to_vec();
        self
    }

    /// Read from `name` environment variable when argument is not present.
    ///
    /// If it is not present in the environment, then default
    /// rules will apply.
    ///
    /// If user sets the argument in the environment:
    /// - When [`Arg::action(ArgAction::Set)`] is not set, the flag is considered raised.
    /// - When [`Arg::action(ArgAction::Set)`] is set,
    ///   [`ArgMatches::get_one`][crate::ArgMatches::get_one] will
    ///   return value of the environment variable.
    ///
    /// If user doesn't set the argument in the environment:
    /// - When [`Arg::action(ArgAction::Set)`] is not set, the flag is considered off.
    /// - When [`Arg::action(ArgAction::Set)`] is set,
    ///   [`ArgMatches::get_one`][crate::ArgMatches::get_one] will
    ///   return the default specified.
    ///
    /// # Examples
    ///
    /// In this example, we show the variable coming from the environment:
    ///
    /// ```rust
    /// # use std::env;
    /// # use clap::{Command, Arg, ArgAction};
    ///
    /// env::set_var("MY_FLAG", "env");
    ///
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .env("MY_FLAG")
    ///         .action(ArgAction::Set))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("flag").unwrap(), "env");
    /// ```
    ///
    /// In this example, because `prog` is a flag that accepts an optional, case-insensitive
    /// boolean literal.
    ///
    /// Note that the value parser controls how flags are parsed.  In this case we've selected
    /// [`FalseyValueParser`][crate::builder::FalseyValueParser].  A `false` literal is `n`, `no`,
    /// `f`, `false`, `off` or `0`.  An absent environment variable will also be considered as
    /// `false`.  Anything else will considered as `true`.
    ///
    /// ```rust
    /// # use std::env;
    /// # use clap::{Command, Arg, ArgAction};
    /// # use clap::builder::FalseyValueParser;
    ///
    /// env::set_var("TRUE_FLAG", "true");
    /// env::set_var("FALSE_FLAG", "0");
    ///
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("true_flag")
    ///         .long("true_flag")
    ///         .action(ArgAction::SetTrue)
    ///         .value_parser(FalseyValueParser::new())
    ///         .env("TRUE_FLAG"))
    ///     .arg(Arg::new("false_flag")
    ///         .long("false_flag")
    ///         .action(ArgAction::SetTrue)
    ///         .value_parser(FalseyValueParser::new())
    ///         .env("FALSE_FLAG"))
    ///     .arg(Arg::new("absent_flag")
    ///         .long("absent_flag")
    ///         .action(ArgAction::SetTrue)
    ///         .value_parser(FalseyValueParser::new())
    ///         .env("ABSENT_FLAG"))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(*m.get_one::<bool>("true_flag").unwrap());
    /// assert!(!*m.get_one::<bool>("false_flag").unwrap());
    /// assert!(!*m.get_one::<bool>("absent_flag").unwrap());
    /// ```
    ///
    /// In this example, we show the variable coming from an option on the CLI:
    ///
    /// ```rust
    /// # use std::env;
    /// # use clap::{Command, Arg, ArgAction};
    ///
    /// env::set_var("MY_FLAG", "env");
    ///
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .env("MY_FLAG")
    ///         .action(ArgAction::Set))
    ///     .get_matches_from(vec![
    ///         "prog", "--flag", "opt"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("flag").unwrap(), "opt");
    /// ```
    ///
    /// In this example, we show the variable coming from the environment even with the
    /// presence of a default:
    ///
    /// ```rust
    /// # use std::env;
    /// # use clap::{Command, Arg, ArgAction};
    ///
    /// env::set_var("MY_FLAG", "env");
    ///
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .env("MY_FLAG")
    ///         .action(ArgAction::Set)
    ///         .default_value("default"))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("flag").unwrap(), "env");
    /// ```
    ///
    /// In this example, we show the use of multiple values in a single environment variable:
    ///
    /// ```rust
    /// # use std::env;
    /// # use clap::{Command, Arg, ArgAction};
    ///
    /// env::set_var("MY_FLAG_MULTI", "env1,env2");
    ///
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .env("MY_FLAG_MULTI")
    ///         .action(ArgAction::Set)
    ///         .num_args(1..)
    ///         .value_delimiter(','))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.get_many::<String>("flag").unwrap().collect::<Vec<_>>(), vec!["env1", "env2"]);
    /// ```
    /// [`Arg::action(ArgAction::Set)`]: Arg::action()
    /// [`Arg::value_delimiter(',')`]: Arg::value_delimiter()
    #[cfg(feature = "env")]
    #[inline]
    #[must_use]
    pub fn env(self, name: &'help str) -> Self {
        self.env_os(OsStr::new(name))
    }

    /// Read from `name` environment variable when argument is not present.
    ///
    /// See [`Arg::env`].
    #[cfg(feature = "env")]
    #[inline]
    #[must_use]
    pub fn env_os(mut self, name: &'help OsStr) -> Self {
        self.env = Some((name, env::var_os(name)));
        self
    }
}

/// # Help
impl<'help> Arg<'help> {
    /// Sets the description of the argument for short help (`-h`).
    ///
    /// Typically, this is a short (one line) description of the arg.
    ///
    /// If [`Arg::long_help`] is not specified, this message will be displayed for `--help`.
    ///
    /// **NOTE:** Only `Arg::help` is used in completion script generation in order to be concise
    ///
    /// # Examples
    ///
    /// Any valid UTF-8 is allowed in the help text. The one exception is when one wishes to
    /// include a newline in the help text and have the following text be properly aligned with all
    /// the other help text.
    ///
    /// Setting `help` displays a short message to the side of the argument when the user passes
    /// `-h` or `--help` (by default).
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays
    ///
    /// ```notrust
    /// helptest
    ///
    /// USAGE:
    ///    helptest [OPTIONS]
    ///
    /// OPTIONS:
    ///     --config     Some help text describing the --config arg
    /// -h, --help       Print help information
    /// -V, --version    Print version information
    /// ```
    /// [`Arg::long_help`]: Arg::long_help()
    #[inline]
    #[must_use]
    pub fn help(mut self, h: impl Into<Option<&'help str>>) -> Self {
        self.help = h.into();
        self
    }

    /// Sets the description of the argument for long help (`--help`).
    ///
    /// Typically this a more detailed (multi-line) message
    /// that describes the arg.
    ///
    /// If [`Arg::help`] is not specified, this message will be displayed for `-h`.
    ///
    /// **NOTE:** Only [`Arg::help`] is used in completion script generation in order to be concise
    ///
    /// # Examples
    ///
    /// Any valid UTF-8 is allowed in the help text. The one exception is when one wishes to
    /// include a newline in the help text and have the following text be properly aligned with all
    /// the other help text.
    ///
    /// Setting `help` displays a short message to the side of the argument when the user passes
    /// `-h` or `--help` (by default).
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .long_help(
    /// "The config file used by the myprog must be in JSON format
    /// with only valid keys and may not contain other nonsense
    /// that cannot be read by this program. Obviously I'm going on
    /// and on, so I'll stop now."))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays
    ///
    /// ```text
    /// prog
    ///
    /// USAGE:
    ///     prog [OPTIONS]
    ///
    /// OPTIONS:
    ///         --config
    ///             The config file used by the myprog must be in JSON format
    ///             with only valid keys and may not contain other nonsense
    ///             that cannot be read by this program. Obviously I'm going on
    ///             and on, so I'll stop now.
    ///
    ///     -h, --help
    ///             Print help information
    ///
    ///     -V, --version
    ///             Print version information
    /// ```
    /// [`Arg::help`]: Arg::help()
    #[inline]
    #[must_use]
    pub fn long_help(mut self, h: impl Into<Option<&'help str>>) -> Self {
        self.long_help = h.into();
        self
    }

    /// Allows custom ordering of args within the help message.
    ///
    /// Args with a lower value will be displayed first in the help message. This is helpful when
    /// one would like to emphasise frequently used args, or prioritize those towards the top of
    /// the list. Args with duplicate display orders will be displayed in alphabetical order.
    ///
    /// **NOTE:** The default is 999 for all arguments.
    ///
    /// **NOTE:** This setting is ignored for [positional arguments] which are always displayed in
    /// [index] order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("a") // Typically args are grouped alphabetically by name.
    ///                              // Args without a display_order have a value of 999 and are
    ///                              // displayed alphabetically with all other 999 valued args.
    ///         .long("long-option")
    ///         .short('o')
    ///         .action(ArgAction::Set)
    ///         .help("Some help and text"))
    ///     .arg(Arg::new("b")
    ///         .long("other-option")
    ///         .short('O')
    ///         .action(ArgAction::Set)
    ///         .display_order(1)   // In order to force this arg to appear *first*
    ///                             // all we have to do is give it a value lower than 999.
    ///                             // Any other args with a value of 1 will be displayed
    ///                             // alphabetically with this one...then 2 values, then 3, etc.
    ///         .help("I should be first!"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays the following help message
    ///
    /// ```text
    /// cust-ord
    ///
    /// USAGE:
    ///     cust-ord [OPTIONS]
    ///
    /// OPTIONS:
    ///     -h, --help                Print help information
    ///     -V, --version             Print version information
    ///     -O, --other-option <b>    I should be first!
    ///     -o, --long-option <a>     Some help and text
    /// ```
    /// [positional arguments]: Arg::index()
    /// [index]: Arg::index()
    #[inline]
    #[must_use]
    pub fn display_order(mut self, ord: usize) -> Self {
        self.disp_ord = Some(ord);
        self
    }

    /// Override the [current] help section.
    ///
    /// [current]: crate::Command::next_help_heading
    #[inline]
    #[must_use]
    pub fn help_heading<O>(mut self, heading: O) -> Self
    where
        O: Into<Option<&'help str>>,
    {
        self.help_heading = Some(heading.into());
        self
    }

    /// Render the [help][Arg::help] on the line after the argument.
    ///
    /// This can be helpful for arguments with very long or complex help messages.
    /// This can also be helpful for arguments with very long flag names, or many/long value names.
    ///
    /// **NOTE:** To apply this setting to all arguments and subcommands, consider using
    /// [`crate::Command::next_line_help`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .long("long-option-flag")
    ///         .short('o')
    ///         .action(ArgAction::Set)
    ///         .next_line_help(true)
    ///         .value_names(&["value1", "value2"])
    ///         .help("Some really long help and complex\n\
    ///                help that makes more sense to be\n\
    ///                on a line after the option"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays the following help message
    ///
    /// ```text
    /// nlh
    ///
    /// USAGE:
    ///     nlh [OPTIONS]
    ///
    /// OPTIONS:
    ///     -h, --help       Print help information
    ///     -V, --version    Print version information
    ///     -o, --long-option-flag <value1> <value2>
    ///         Some really long help and complex
    ///         help that makes more sense to be
    ///         on a line after the option
    /// ```
    #[inline]
    #[must_use]
    pub fn next_line_help(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::NextLineHelp)
        } else {
            self.unset_setting(ArgSettings::NextLineHelp)
        }
    }

    /// Do not display the argument in help message.
    ///
    /// **NOTE:** This does **not** hide the argument from usage strings on error
    ///
    /// # Examples
    ///
    /// Setting `Hidden` will hide the argument when displaying help text
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .hide(true)
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays
    ///
    /// ```text
    /// helptest
    ///
    /// USAGE:
    ///    helptest [OPTIONS]
    ///
    /// OPTIONS:
    /// -h, --help       Print help information
    /// -V, --version    Print version information
    /// ```
    #[inline]
    #[must_use]
    pub fn hide(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::Hidden)
        } else {
            self.unset_setting(ArgSettings::Hidden)
        }
    }

    /// Do not display the [possible values][crate::builder::ValueParser::possible_values] in the help message.
    ///
    /// This is useful for args with many values, or ones which are explained elsewhere in the
    /// help text.
    ///
    /// **NOTE:** Setting this requires [taking values][Arg::num_args]
    ///
    /// To set this for all arguments, see
    /// [`Command::hide_possible_values`][crate::Command::hide_possible_values].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .long("mode")
    ///         .value_parser(["fast", "slow"])
    ///         .action(ArgAction::Set)
    ///         .hide_possible_values(true));
    /// ```
    /// If we were to run the above program with `--help` the `[values: fast, slow]` portion of
    /// the help text would be omitted.
    #[inline]
    #[must_use]
    pub fn hide_possible_values(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::HidePossibleValues)
        } else {
            self.unset_setting(ArgSettings::HidePossibleValues)
        }
    }

    /// Do not display the default value of the argument in the help message.
    ///
    /// This is useful when default behavior of an arg is explained elsewhere in the help text.
    ///
    /// **NOTE:** Setting this requires [taking values][Arg::num_args]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("connect")
    ///     .arg(Arg::new("host")
    ///         .long("host")
    ///         .default_value("localhost")
    ///         .action(ArgAction::Set)
    ///         .hide_default_value(true));
    ///
    /// ```
    ///
    /// If we were to run the above program with `--help` the `[default: localhost]` portion of
    /// the help text would be omitted.
    #[inline]
    #[must_use]
    pub fn hide_default_value(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::HideDefaultValue)
        } else {
            self.unset_setting(ArgSettings::HideDefaultValue)
        }
    }

    /// Do not display in help the environment variable name.
    ///
    /// This is useful when the variable option is explained elsewhere in the help text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .long("mode")
    ///         .env("MODE")
    ///         .action(ArgAction::Set)
    ///         .hide_env(true));
    /// ```
    ///
    /// If we were to run the above program with `--help` the `[env: MODE]` portion of the help
    /// text would be omitted.
    #[cfg(feature = "env")]
    #[inline]
    #[must_use]
    pub fn hide_env(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::HideEnv)
        } else {
            self.unset_setting(ArgSettings::HideEnv)
        }
    }

    /// Do not display in help any values inside the associated ENV variables for the argument.
    ///
    /// This is useful when ENV vars contain sensitive values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("connect")
    ///     .arg(Arg::new("host")
    ///         .long("host")
    ///         .env("CONNECT")
    ///         .action(ArgAction::Set)
    ///         .hide_env_values(true));
    ///
    /// ```
    ///
    /// If we were to run the above program with `$ CONNECT=super_secret connect --help` the
    /// `[default: CONNECT=super_secret]` portion of the help text would be omitted.
    #[cfg(feature = "env")]
    #[inline]
    #[must_use]
    pub fn hide_env_values(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::HideEnvValues)
        } else {
            self.unset_setting(ArgSettings::HideEnvValues)
        }
    }

    /// Hides an argument from short help (`-h`).
    ///
    /// **NOTE:** This does **not** hide the argument from usage strings on error
    ///
    /// **NOTE:** Setting this option will cause next-line-help output style to be used
    /// when long help (`--help`) is called.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// Arg::new("debug")
    ///     .hide_short_help(true);
    /// ```
    ///
    /// Setting `hide_short_help(true)` will hide the argument when displaying short help text
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .hide_short_help(true)
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "-h"
    ///     ]);
    /// ```
    ///
    /// The above example displays
    ///
    /// ```text
    /// helptest
    ///
    /// USAGE:
    ///    helptest [OPTIONS]
    ///
    /// OPTIONS:
    /// -h, --help       Print help information
    /// -V, --version    Print version information
    /// ```
    ///
    /// However, when --help is called
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .hide_short_help(true)
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// Then the following would be displayed
    ///
    /// ```text
    /// helptest
    ///
    /// USAGE:
    ///    helptest [OPTIONS]
    ///
    /// OPTIONS:
    ///     --config     Some help text describing the --config arg
    /// -h, --help       Print help information
    /// -V, --version    Print version information
    /// ```
    #[inline]
    #[must_use]
    pub fn hide_short_help(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::HiddenShortHelp)
        } else {
            self.unset_setting(ArgSettings::HiddenShortHelp)
        }
    }

    /// Hides an argument from long help (`--help`).
    ///
    /// **NOTE:** This does **not** hide the argument from usage strings on error
    ///
    /// **NOTE:** Setting this option will cause next-line-help output style to be used
    /// when long help (`--help`) is called.
    ///
    /// # Examples
    ///
    /// Setting `hide_long_help(true)` will hide the argument when displaying long help text
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .hide_long_help(true)
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays
    ///
    /// ```text
    /// helptest
    ///
    /// USAGE:
    ///    helptest [OPTIONS]
    ///
    /// OPTIONS:
    /// -h, --help       Print help information
    /// -V, --version    Print version information
    /// ```
    ///
    /// However, when -h is called
    ///
    /// ```rust
    /// # use clap::{Command, Arg};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .hide_long_help(true)
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "-h"
    ///     ]);
    /// ```
    ///
    /// Then the following would be displayed
    ///
    /// ```text
    /// helptest
    ///
    /// USAGE:
    ///    helptest [OPTIONS]
    ///
    /// OPTIONS:
    ///     --config     Some help text describing the --config arg
    /// -h, --help       Print help information
    /// -V, --version    Print version information
    /// ```
    #[inline]
    #[must_use]
    pub fn hide_long_help(self, yes: bool) -> Self {
        if yes {
            self.setting(ArgSettings::HiddenLongHelp)
        } else {
            self.unset_setting(ArgSettings::HiddenLongHelp)
        }
    }
}

/// # Advanced Argument Relations
impl<'help> Arg<'help> {
    /// The name of the [`ArgGroup`] the argument belongs to.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// Arg::new("debug")
    ///     .long("debug")
    ///     .action(ArgAction::SetTrue)
    ///     .group("mode")
    /// # ;
    /// ```
    ///
    /// Multiple arguments can be a member of a single group and then the group checked as if it
    /// was one of said arguments.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("debug")
    ///         .long("debug")
    ///         .action(ArgAction::SetTrue)
    ///         .group("mode"))
    ///     .arg(Arg::new("verbose")
    ///         .long("verbose")
    ///         .action(ArgAction::SetTrue)
    ///         .group("mode"))
    ///     .get_matches_from(vec![
    ///         "prog", "--debug"
    ///     ]);
    /// assert!(m.contains_id("mode"));
    /// ```
    ///
    /// [`ArgGroup`]: crate::ArgGroup
    #[must_use]
    pub fn group<T: Key>(mut self, group_id: T) -> Self {
        self.groups.push(group_id.into());
        self
    }

    /// The names of [`ArgGroup`]'s the argument belongs to.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// Arg::new("debug")
    ///     .long("debug")
    ///     .action(ArgAction::SetTrue)
    ///     .groups(&["mode", "verbosity"])
    /// # ;
    /// ```
    ///
    /// Arguments can be members of multiple groups and then the group checked as if it
    /// was one of said arguments.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("debug")
    ///         .long("debug")
    ///         .action(ArgAction::SetTrue)
    ///         .groups(&["mode", "verbosity"]))
    ///     .arg(Arg::new("verbose")
    ///         .long("verbose")
    ///         .action(ArgAction::SetTrue)
    ///         .groups(&["mode", "verbosity"]))
    ///     .get_matches_from(vec![
    ///         "prog", "--debug"
    ///     ]);
    /// assert!(m.contains_id("mode"));
    /// assert!(m.contains_id("verbosity"));
    /// ```
    ///
    /// [`ArgGroup`]: crate::ArgGroup
    #[must_use]
    pub fn groups<T: Key>(mut self, group_ids: &[T]) -> Self {
        self.groups.extend(group_ids.iter().map(Id::from));
        self
    }

    /// Specifies the value of the argument if `arg` has been used at runtime.
    ///
    /// If `val` is set to `None`, `arg` only needs to be present. If `val` is set to `"some-val"`
    /// then `arg` must be present at runtime **and** have the value `val`.
    ///
    /// If `default` is set to `None`, `default_value` will be removed.
    ///
    /// **NOTE:** This setting is perfectly compatible with [`Arg::default_value`] but slightly
    /// different. `Arg::default_value` *only* takes effect when the user has not provided this arg
    /// at runtime. This setting however only takes effect when the user has not provided a value at
    /// runtime **and** these other conditions are met as well. If you have set `Arg::default_value`
    /// and `Arg::default_value_if`, and the user **did not** provide this arg at runtime, nor were
    /// the conditions met for `Arg::default_value_if`, the `Arg::default_value` will be applied.
    ///
    /// **NOTE:** This implicitly sets [`Arg::action(ArgAction::Set)`].
    ///
    /// # Examples
    ///
    /// First we use the default value only if another arg is present at runtime.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .action(ArgAction::SetTrue))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_if("flag", None, Some("default")))
    ///     .get_matches_from(vec![
    ///         "prog", "--flag"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("other").unwrap(), "default");
    /// ```
    ///
    /// Next we run the same test, but without providing `--flag`.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .action(ArgAction::SetTrue))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_if("flag", Some("true"), Some("default")))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("other"), None);
    /// ```
    ///
    /// Now lets only use the default value if `--opt` contains the value `special`.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .action(ArgAction::Set)
    ///         .long("opt"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_if("opt", Some("special"), Some("default")))
    ///     .get_matches_from(vec![
    ///         "prog", "--opt", "special"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("other").unwrap(), "default");
    /// ```
    ///
    /// We can run the same test and provide any value *other than* `special` and we won't get a
    /// default value.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .action(ArgAction::Set)
    ///         .long("opt"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_if("opt", Some("special"), Some("default")))
    ///     .get_matches_from(vec![
    ///         "prog", "--opt", "hahaha"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("other"), None);
    /// ```
    ///
    /// If we want to unset the default value for an Arg based on the presence or
    /// value of some other Arg.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .action(ArgAction::SetTrue))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value("default")
    ///         .default_value_if("flag", Some("true"), None))
    ///     .get_matches_from(vec![
    ///         "prog", "--flag"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("other"), None);
    /// ```
    /// [`Arg::action(ArgAction::Set)`]: Arg::action()
    /// [`Arg::default_value`]: Arg::default_value()
    #[must_use]
    pub fn default_value_if<T: Key>(
        self,
        arg_id: T,
        val: Option<&'help str>,
        default: Option<&'help str>,
    ) -> Self {
        self.default_value_if_os(arg_id, val.map(OsStr::new), default.map(OsStr::new))
    }

    /// Provides a conditional default value in the exact same manner as [`Arg::default_value_if`]
    /// only using [`OsStr`]s instead.
    ///
    /// [`Arg::default_value_if`]: Arg::default_value_if()
    /// [`OsStr`]: std::ffi::OsStr
    #[must_use]
    pub fn default_value_if_os<T: Key>(
        mut self,
        arg_id: T,
        val: Option<&'help OsStr>,
        default: Option<&'help OsStr>,
    ) -> Self {
        self.default_vals_ifs
            .push((arg_id.into(), val.into(), default));
        self
    }

    /// Specifies multiple values and conditions in the same manner as [`Arg::default_value_if`].
    ///
    /// The method takes a slice of tuples in the `(arg, Option<val>, default)` format.
    ///
    /// **NOTE**: The conditions are stored in order and evaluated in the same order. I.e. the first
    /// if multiple conditions are true, the first one found will be applied and the ultimate value.
    ///
    /// # Examples
    ///
    /// First we use the default value only if another arg is present at runtime.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .action(ArgAction::SetTrue))
    ///     .arg(Arg::new("opt")
    ///         .long("opt")
    ///         .action(ArgAction::Set))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_ifs(&[
    ///             ("flag", Some("true"), Some("default")),
    ///             ("opt", Some("channal"), Some("chan")),
    ///         ]))
    ///     .get_matches_from(vec![
    ///         "prog", "--opt", "channal"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("other").unwrap(), "chan");
    /// ```
    ///
    /// Next we run the same test, but without providing `--flag`.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .action(ArgAction::SetTrue))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_ifs(&[
    ///             ("flag", Some("true"), Some("default")),
    ///             ("opt", Some("channal"), Some("chan")),
    ///         ]))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("other"), None);
    /// ```
    ///
    /// We can also see that these values are applied in order, and if more than one condition is
    /// true, only the first evaluated "wins"
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let m = Command::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .action(ArgAction::SetTrue))
    ///     .arg(Arg::new("opt")
    ///         .long("opt")
    ///         .action(ArgAction::Set))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_ifs(&[
    ///             ("flag", None, Some("default")),
    ///             ("opt", Some("channal"), Some("chan")),
    ///         ]))
    ///     .get_matches_from(vec![
    ///         "prog", "--opt", "channal", "--flag"
    ///     ]);
    ///
    /// assert_eq!(m.get_one::<String>("other").unwrap(), "default");
    /// ```
    /// [`Arg::action(ArgAction::Set)`]: Arg::action()
    /// [`Arg::default_value_if`]: Arg::default_value_if()
    #[must_use]
    pub fn default_value_ifs<T: Key>(
        mut self,
        ifs: &[(T, Option<&'help str>, Option<&'help str>)],
    ) -> Self {
        for (arg, val, default) in ifs {
            self = self.default_value_if_os(arg, val.map(OsStr::new), default.map(OsStr::new));
        }
        self
    }

    /// Provides multiple conditional default values in the exact same manner as
    /// [`Arg::default_value_ifs`] only using [`OsStr`]s instead.
    ///
    /// [`Arg::default_value_ifs`]: Arg::default_value_ifs()
    /// [`OsStr`]: std::ffi::OsStr
    #[must_use]
    pub fn default_value_ifs_os<T: Key>(
        mut self,
        ifs: &[(T, Option<&'help OsStr>, Option<&'help OsStr>)],
    ) -> Self {
        for (arg, val, default) in ifs {
            self = self.default_value_if_os(arg, *val, *default);
        }
        self
    }

    /// Set this arg as [required] as long as the specified argument is not present at runtime.
    ///
    /// **Pro Tip:** Using `Arg::required_unless_present` implies [`Arg::required`] and is therefore not
    /// mandatory to also set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_unless_present("debug")
    /// # ;
    /// ```
    ///
    /// In the following example, the required argument is *not* provided,
    /// but it's not an error because the `unless` arg has been supplied.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless_present("dbg")
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug")
    ///         .action(ArgAction::SetTrue))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--debug"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// Setting `Arg::required_unless_present(name)` and *not* supplying `name` or this arg is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless_present("dbg")
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug"))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    /// [required]: Arg::required()
    #[must_use]
    pub fn required_unless_present<T: Key>(mut self, arg_id: T) -> Self {
        self.r_unless.push(arg_id.into());
        self
    }

    /// Sets this arg as [required] unless *all* of the specified arguments are present at runtime.
    ///
    /// In other words, parsing will succeed only if user either
    /// * supplies the `self` arg.
    /// * supplies *all* of the `names` arguments.
    ///
    /// **NOTE:** If you wish for this argument to only be required unless *any of* these args are
    /// present see [`Arg::required_unless_present_any`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_unless_present_all(&["cfg", "dbg"])
    /// # ;
    /// ```
    ///
    /// In the following example, the required argument is *not* provided, but it's not an error
    /// because *all* of the `names` args have been supplied.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless_present_all(&["dbg", "infile"])
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug")
    ///         .action(ArgAction::SetTrue))
    ///     .arg(Arg::new("infile")
    ///         .short('i')
    ///         .action(ArgAction::Set))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--debug", "-i", "file"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// Setting [`Arg::required_unless_present_all(names)`] and *not* supplying
    /// either *all* of `unless` args or the `self` arg is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless_present_all(&["dbg", "infile"])
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug")
    ///         .action(ArgAction::SetTrue))
    ///     .arg(Arg::new("infile")
    ///         .short('i')
    ///         .action(ArgAction::Set))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    /// [required]: Arg::required()
    /// [`Arg::required_unless_present_any`]: Arg::required_unless_present_any()
    /// [`Arg::required_unless_present_all(names)`]: Arg::required_unless_present_all()
    #[must_use]
    pub fn required_unless_present_all<T, I>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Key,
    {
        self.r_unless_all.extend(names.into_iter().map(Id::from));
        self
    }

    /// Sets this arg as [required] unless *any* of the specified arguments are present at runtime.
    ///
    /// In other words, parsing will succeed only if user either
    /// * supplies the `self` arg.
    /// * supplies *one or more* of the `unless` arguments.
    ///
    /// **NOTE:** If you wish for this argument to be required unless *all of* these args are
    /// present see [`Arg::required_unless_present_all`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_unless_present_any(&["cfg", "dbg"])
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::required_unless_present_any(names)`] requires that the argument be used at runtime
    /// *unless* *at least one of* the args in `names` are present. In the following example, the
    /// required argument is *not* provided, but it's not an error because one the `unless` args
    /// have been supplied.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless_present_any(&["dbg", "infile"])
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug")
    ///         .action(ArgAction::SetTrue))
    ///     .arg(Arg::new("infile")
    ///         .short('i')
    ///         .action(ArgAction::Set))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--debug"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// Setting [`Arg::required_unless_present_any(names)`] and *not* supplying *at least one of* `names`
    /// or this arg is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless_present_any(&["dbg", "infile"])
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug")
    ///         .action(ArgAction::SetTrue))
    ///     .arg(Arg::new("infile")
    ///         .short('i')
    ///         .action(ArgAction::Set))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    /// [required]: Arg::required()
    /// [`Arg::required_unless_present_any(names)`]: Arg::required_unless_present_any()
    /// [`Arg::required_unless_present_all`]: Arg::required_unless_present_all()
    #[must_use]
    pub fn required_unless_present_any<T, I>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Key,
    {
        self.r_unless.extend(names.into_iter().map(Id::from));
        self
    }

    /// This argument is [required] only if the specified `arg` is present at runtime and its value
    /// equals `val`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_if_eq("other_arg", "value")
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .required_if_eq("other", "special")
    ///         .long("config"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .action(ArgAction::Set))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--other", "not-special"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use --other=special, so "cfg" wasn't required
    ///
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .required_if_eq("other", "special")
    ///         .long("config"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .action(ArgAction::Set))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--other", "special"
    ///     ]);
    ///
    /// // We did use --other=special so "cfg" had become required but was missing.
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    ///
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .required_if_eq("other", "special")
    ///         .long("config"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .action(ArgAction::Set))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--other", "SPECIAL"
    ///     ]);
    ///
    /// // By default, the comparison is case-sensitive, so "cfg" wasn't required
    /// assert!(res.is_ok());
    ///
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .required_if_eq("other", "special")
    ///         .long("config"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .ignore_case(true)
    ///         .action(ArgAction::Set))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--other", "SPECIAL"
    ///     ]);
    ///
    /// // However, case-insensitive comparisons can be enabled.  This typically occurs when using Arg::possible_values().
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::requires(name)`]: Arg::requires()
    /// [Conflicting]: Arg::conflicts_with()
    /// [required]: Arg::required()
    #[must_use]
    pub fn required_if_eq<T: Key>(mut self, arg_id: T, val: &'help str) -> Self {
        self.r_ifs.push((arg_id.into(), val));
        self
    }

    /// Specify this argument is [required] based on multiple conditions.
    ///
    /// The conditions are set up in a `(arg, val)` style tuple. The requirement will only become
    /// valid if one of the specified `arg`'s value equals its corresponding `val`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_if_eq_any(&[
    ///         ("extra", "val"),
    ///         ("option", "spec")
    ///     ])
    /// # ;
    /// ```
    ///
    /// Setting `Arg::required_if_eq_any(&[(arg, val)])` makes this arg required if any of the `arg`s
    /// are used at runtime and it's corresponding value is equal to `val`. If the `arg`'s value is
    /// anything other than `val`, this argument isn't required.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_if_eq_any(&[
    ///             ("extra", "val"),
    ///             ("option", "spec")
    ///         ])
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .arg(Arg::new("extra")
    ///         .action(ArgAction::Set)
    ///         .long("extra"))
    ///     .arg(Arg::new("option")
    ///         .action(ArgAction::Set)
    ///         .long("option"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--option", "other"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use --option=spec, or --extra=val so "cfg" isn't required
    /// ```
    ///
    /// Setting `Arg::required_if_eq_any(&[(arg, val)])` and having any of the `arg`s used with its
    /// value of `val` but *not* using this arg is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_if_eq_any(&[
    ///             ("extra", "val"),
    ///             ("option", "spec")
    ///         ])
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .arg(Arg::new("extra")
    ///         .action(ArgAction::Set)
    ///         .long("extra"))
    ///     .arg(Arg::new("option")
    ///         .action(ArgAction::Set)
    ///         .long("option"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--option", "spec"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::requires(name)`]: Arg::requires()
    /// [Conflicting]: Arg::conflicts_with()
    /// [required]: Arg::required()
    #[must_use]
    pub fn required_if_eq_any<T: Key>(mut self, ifs: &[(T, &'help str)]) -> Self {
        self.r_ifs
            .extend(ifs.iter().map(|(id, val)| (Id::from_ref(id), *val)));
        self
    }

    /// Specify this argument is [required] based on multiple conditions.
    ///
    /// The conditions are set up in a `(arg, val)` style tuple. The requirement will only become
    /// valid if every one of the specified `arg`'s value equals its corresponding `val`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_if_eq_all(&[
    ///         ("extra", "val"),
    ///         ("option", "spec")
    ///     ])
    /// # ;
    /// ```
    ///
    /// Setting `Arg::required_if_eq_all(&[(arg, val)])` makes this arg required if all of the `arg`s
    /// are used at runtime and every value is equal to its corresponding `val`. If the `arg`'s value is
    /// anything other than `val`, this argument isn't required.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_if_eq_all(&[
    ///             ("extra", "val"),
    ///             ("option", "spec")
    ///         ])
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .arg(Arg::new("extra")
    ///         .action(ArgAction::Set)
    ///         .long("extra"))
    ///     .arg(Arg::new("option")
    ///         .action(ArgAction::Set)
    ///         .long("option"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--option", "spec"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use --option=spec --extra=val so "cfg" isn't required
    /// ```
    ///
    /// Setting `Arg::required_if_eq_all(&[(arg, val)])` and having all of the `arg`s used with its
    /// value of `val` but *not* using this arg is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_if_eq_all(&[
    ///             ("extra", "val"),
    ///             ("option", "spec")
    ///         ])
    ///         .action(ArgAction::Set)
    ///         .long("config"))
    ///     .arg(Arg::new("extra")
    ///         .action(ArgAction::Set)
    ///         .long("extra"))
    ///     .arg(Arg::new("option")
    ///         .action(ArgAction::Set)
    ///         .long("option"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--extra", "val", "--option", "spec"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    /// [required]: Arg::required()
    #[must_use]
    pub fn required_if_eq_all<T: Key>(mut self, ifs: &[(T, &'help str)]) -> Self {
        self.r_ifs_all
            .extend(ifs.iter().map(|(id, val)| (Id::from_ref(id), *val)));
        self
    }

    /// Require another argument if this arg was present at runtime and its value equals to `val`.
    ///
    /// This method takes `value, another_arg` pair. At runtime, clap will check
    /// if this arg (`self`) is present and its value equals to `val`.
    /// If it does, `another_arg` will be marked as required.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .requires_if("val", "arg")
    /// # ;
    /// ```
    ///
    /// Setting `Arg::requires_if(val, arg)` requires that the `arg` be used at runtime if the
    /// defining argument's value is equal to `val`. If the defining argument is anything other than
    /// `val`, the other argument isn't required.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .requires_if("my.cfg", "other")
    ///         .long("config"))
    ///     .arg(Arg::new("other"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "some.cfg"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use --config=my.cfg, so other wasn't required
    /// ```
    ///
    /// Setting `Arg::requires_if(val, arg)` and setting the value to `val` but *not* supplying
    /// `arg` is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .requires_if("my.cfg", "input")
    ///         .long("config"))
    ///     .arg(Arg::new("input"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "my.cfg"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::requires(name)`]: Arg::requires()
    /// [Conflicting]: Arg::conflicts_with()
    /// [override]: Arg::overrides_with()
    #[must_use]
    pub fn requires_if<T: Key>(mut self, val: &'help str, arg_id: T) -> Self {
        self.requires
            .push((ArgPredicate::Equals(OsStr::new(val)), arg_id.into()));
        self
    }

    /// Allows multiple conditional requirements.
    ///
    /// The requirement will only become valid if this arg's value equals `val`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .requires_ifs(&[
    ///         ("val", "arg"),
    ///         ("other_val", "arg2"),
    ///     ])
    /// # ;
    /// ```
    ///
    /// Setting `Arg::requires_ifs(&["val", "arg"])` requires that the `arg` be used at runtime if the
    /// defining argument's value is equal to `val`. If the defining argument's value is anything other
    /// than `val`, `arg` isn't required.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .requires_ifs(&[
    ///             ("special.conf", "opt"),
    ///             ("other.conf", "other"),
    ///         ])
    ///         .long("config"))
    ///     .arg(Arg::new("opt")
    ///         .long("option")
    ///         .action(ArgAction::Set))
    ///     .arg(Arg::new("other"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "special.conf"
    ///     ]);
    ///
    /// assert!(res.is_err()); // We  used --config=special.conf so --option <val> is required
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::requires(name)`]: Arg::requires()
    /// [Conflicting]: Arg::conflicts_with()
    /// [override]: Arg::overrides_with()
    #[must_use]
    pub fn requires_ifs<T: Key>(mut self, ifs: &[(&'help str, T)]) -> Self {
        self.requires.extend(
            ifs.iter()
                .map(|(val, arg)| (ArgPredicate::Equals(OsStr::new(*val)), Id::from(arg))),
        );
        self
    }

    /// Require these arguments names when this one is present
    ///
    /// i.e. when using this argument, the following arguments *must* be present.
    ///
    /// **NOTE:** [Conflicting] rules and [override] rules take precedence over being required
    /// by default.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .requires_all(&["input", "output"])
    /// # ;
    /// ```
    ///
    /// Setting `Arg::requires_all(&[arg, arg2])` requires that all the arguments be used at
    /// runtime if the defining argument is used. If the defining argument isn't used, the other
    /// argument isn't required
    ///
    /// ```rust
    /// # use clap::{Command, Arg, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .requires("input")
    ///         .long("config"))
    ///     .arg(Arg::new("input"))
    ///     .arg(Arg::new("output"))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use cfg, so input and output weren't required
    /// ```
    ///
    /// Setting `Arg::requires_all(&[arg, arg2])` and *not* supplying all the arguments is an
    /// error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .requires_all(&["input", "output"])
    ///         .long("config"))
    ///     .arg(Arg::new("input"))
    ///     .arg(Arg::new("output"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "file.conf", "in.txt"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// // We didn't use output
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
    /// ```
    /// [Conflicting]: Arg::conflicts_with()
    /// [override]: Arg::overrides_with()
    #[must_use]
    pub fn requires_all<T: Key>(mut self, names: &[T]) -> Self {
        self.requires
            .extend(names.iter().map(|s| (ArgPredicate::IsPresent, s.into())));
        self
    }

    /// This argument is mutually exclusive with the specified argument.
    ///
    /// **NOTE:** Conflicting rules take precedence over being required by default. Conflict rules
    /// only need to be set for one of the two arguments, they do not need to be set for each.
    ///
    /// **NOTE:** Defining a conflict is two-way, but does *not* need to defined for both arguments
    /// (i.e. if A conflicts with B, defining A.conflicts_with(B) is sufficient. You do not
    /// need to also do B.conflicts_with(A))
    ///
    /// **NOTE:** [`Arg::conflicts_with_all(names)`] allows specifying an argument which conflicts with more than one argument.
    ///
    /// **NOTE** [`Arg::exclusive(true)`] allows specifying an argument which conflicts with every other argument.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .conflicts_with("debug")
    /// # ;
    /// ```
    ///
    /// Setting conflicting argument, and having both arguments present at runtime is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .conflicts_with("debug")
    ///         .long("config"))
    ///     .arg(Arg::new("debug")
    ///         .long("debug")
    ///         .action(ArgAction::SetTrue))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--debug", "--config", "file.conf"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::ArgumentConflict);
    /// ```
    ///
    /// [`Arg::conflicts_with_all(names)`]: Arg::conflicts_with_all()
    /// [`Arg::exclusive(true)`]: Arg::exclusive()
    #[must_use]
    pub fn conflicts_with<T: Key>(mut self, arg_id: T) -> Self {
        self.blacklist.push(arg_id.into());
        self
    }

    /// This argument is mutually exclusive with the specified arguments.
    ///
    /// See [`Arg::conflicts_with`].
    ///
    /// **NOTE:** Conflicting rules take precedence over being required by default. Conflict rules
    /// only need to be set for one of the two arguments, they do not need to be set for each.
    ///
    /// **NOTE:** Defining a conflict is two-way, but does *not* need to defined for both arguments
    /// (i.e. if A conflicts with B, defining A.conflicts_with(B) is sufficient. You do not need
    /// need to also do B.conflicts_with(A))
    ///
    /// **NOTE:** [`Arg::exclusive(true)`] allows specifying an argument which conflicts with every other argument.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .conflicts_with_all(&["debug", "input"])
    /// # ;
    /// ```
    ///
    /// Setting conflicting argument, and having any of the arguments present at runtime with a
    /// conflicting argument is an error.
    ///
    /// ```rust
    /// # use clap::{Command, Arg, error::ErrorKind, ArgAction};
    /// let res = Command::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .action(ArgAction::Set)
    ///         .conflicts_with_all(&["debug", "input"])
    ///         .long("config"))
    ///     .arg(Arg::new("debug")
    ///         .long("debug"))
    ///     .arg(Arg::new("input"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "file.conf", "file.txt"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind(), ErrorKind::ArgumentConflict);
    /// ```
    /// [`Arg::conflicts_with`]: Arg::conflicts_with()
    /// [`Arg::exclusive(true)`]: Arg::exclusive()
    #[must_use]
    pub fn conflicts_with_all(mut self, names: &[&str]) -> Self {
        self.blacklist.extend(names.iter().copied().map(Id::from));
        self
    }

    /// Sets an overridable argument.
    ///
    /// i.e. this argument and the following argument
    /// will override each other in POSIX style (whichever argument was specified at runtime
    /// **last** "wins")
    ///
    /// **NOTE:** When an argument is overridden it is essentially as if it never was used, any
    /// conflicts, requirements, etc. are evaluated **after** all "overrides" have been removed
    ///
    /// **NOTE:** Overriding an argument implies they [conflict][Arg::conflicts_with`].
    ///
    /// **NOTE:** All arguments implicitly override themselves.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, arg};
    /// let m = Command::new("prog")
    ///     .arg(arg!(-f --flag "some flag")
    ///         .conflicts_with("debug"))
    ///     .arg(arg!(-d --debug "other flag"))
    ///     .arg(arg!(-c --color "third flag")
    ///         .overrides_with("flag"))
    ///     .get_matches_from(vec![
    ///         "prog", "-f", "-d", "-c"]);
    ///             //    ^~~~~~~~~~~~^~~~~ flag is overridden by color
    ///
    /// assert!(*m.get_one::<bool>("color").unwrap());
    /// assert!(*m.get_one::<bool>("debug").unwrap()); // even though flag conflicts with debug, it's as if flag
    ///                                 // was never used because it was overridden with color
    /// assert!(!*m.get_one::<bool>("flag").unwrap());
    /// ```
    #[must_use]
    pub fn overrides_with<T: Key>(mut self, arg_id: T) -> Self {
        self.overrides.push(arg_id.into());
        self
    }

    /// Sets multiple mutually overridable arguments by name.
    ///
    /// i.e. this argument and the following argument will override each other in POSIX style
    /// (whichever argument was specified at runtime **last** "wins")
    ///
    /// **NOTE:** When an argument is overridden it is essentially as if it never was used, any
    /// conflicts, requirements, etc. are evaluated **after** all "overrides" have been removed
    ///
    /// **NOTE:** Overriding an argument implies they [conflict][Arg::conflicts_with_all`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Command, arg};
    /// let m = Command::new("prog")
    ///     .arg(arg!(-f --flag "some flag")
    ///         .conflicts_with("color"))
    ///     .arg(arg!(-d --debug "other flag"))
    ///     .arg(arg!(-c --color "third flag")
    ///         .overrides_with_all(&["flag", "debug"]))
    ///     .get_matches_from(vec![
    ///         "prog", "-f", "-d", "-c"]);
    ///             //    ^~~~~~^~~~~~~~~ flag and debug are overridden by color
    ///
    /// assert!(*m.get_one::<bool>("color").unwrap()); // even though flag conflicts with color, it's as if flag
    ///                                 // and debug were never used because they were overridden
    ///                                 // with color
    /// assert!(!*m.get_one::<bool>("debug").unwrap());
    /// assert!(!*m.get_one::<bool>("flag").unwrap());
    /// ```
    #[must_use]
    pub fn overrides_with_all<T: Key>(mut self, names: &[T]) -> Self {
        self.overrides.extend(names.iter().map(Id::from));
        self
    }
}

/// # Reflection
impl<'help> Arg<'help> {
    /// Get the name of the argument
    #[inline]
    pub fn get_id(&self) -> &'help str {
        self.name
    }

    /// Get the help specified for this argument, if any
    #[inline]
    pub fn get_help(&self) -> Option<&'help str> {
        self.help
    }

    /// Get the long help specified for this argument, if any
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// let arg = Arg::new("foo").long_help("long help");
    /// assert_eq!(Some("long help"), arg.get_long_help());
    /// ```
    ///
    #[inline]
    pub fn get_long_help(&self) -> Option<&'help str> {
        self.long_help
    }

    /// Get the help heading specified for this argument, if any
    #[inline]
    pub fn get_help_heading(&self) -> Option<&'help str> {
        self.help_heading.unwrap_or_default()
    }

    /// Get the short option name for this argument, if any
    #[inline]
    pub fn get_short(&self) -> Option<char> {
        self.short
    }

    /// Get visible short aliases for this argument, if any
    #[inline]
    pub fn get_visible_short_aliases(&self) -> Option<Vec<char>> {
        if self.short_aliases.is_empty() {
            None
        } else {
            Some(
                self.short_aliases
                    .iter()
                    .filter_map(|(c, v)| if *v { Some(c) } else { None })
                    .copied()
                    .collect(),
            )
        }
    }

    /// Get *all* short aliases for this argument, if any, both visible and hidden.
    #[inline]
    pub fn get_all_short_aliases(&self) -> Option<Vec<char>> {
        if self.short_aliases.is_empty() {
            None
        } else {
            Some(self.short_aliases.iter().map(|(s, _)| s).copied().collect())
        }
    }

    /// Get the short option name and its visible aliases, if any
    #[inline]
    pub fn get_short_and_visible_aliases(&self) -> Option<Vec<char>> {
        let mut shorts = match self.short {
            Some(short) => vec![short],
            None => return None,
        };
        if let Some(aliases) = self.get_visible_short_aliases() {
            shorts.extend(aliases);
        }
        Some(shorts)
    }

    /// Get the long option name for this argument, if any
    #[inline]
    pub fn get_long(&self) -> Option<&'help str> {
        self.long
    }

    /// Get visible aliases for this argument, if any
    #[inline]
    pub fn get_visible_aliases(&self) -> Option<Vec<&'help str>> {
        if self.aliases.is_empty() {
            None
        } else {
            Some(
                self.aliases
                    .iter()
                    .filter_map(|(s, v)| if *v { Some(s) } else { None })
                    .copied()
                    .collect(),
            )
        }
    }

    /// Get *all* aliases for this argument, if any, both visible and hidden.
    #[inline]
    pub fn get_all_aliases(&self) -> Option<Vec<&'help str>> {
        if self.aliases.is_empty() {
            None
        } else {
            Some(self.aliases.iter().map(|(s, _)| s).copied().collect())
        }
    }

    /// Get the long option name and its visible aliases, if any
    #[inline]
    pub fn get_long_and_visible_aliases(&self) -> Option<Vec<&'help str>> {
        let mut longs = match self.long {
            Some(long) => vec![long],
            None => return None,
        };
        if let Some(aliases) = self.get_visible_aliases() {
            longs.extend(aliases);
        }
        Some(longs)
    }

    pub(crate) fn get_possible_values(&self) -> Vec<PossibleValue<'help>> {
        if !self.is_takes_value_set() {
            vec![]
        } else {
            self.get_value_parser()
                .possible_values()
                .map(|pvs| pvs.collect())
                .unwrap_or_default()
        }
    }

    /// Get the names of values for this argument.
    #[inline]
    pub fn get_value_names(&self) -> Option<&[&'help str]> {
        if self.val_names.is_empty() {
            None
        } else {
            Some(&self.val_names)
        }
    }

    /// Get the number of values for this argument.
    #[inline]
    pub fn get_num_args(&self) -> Option<ValueRange> {
        self.num_vals
    }

    #[inline]
    pub(crate) fn get_min_vals(&self) -> usize {
        self.get_num_args().expect(INTERNAL_ERROR_MSG).min_values()
    }

    /// Get the delimiter between multiple values
    #[inline]
    pub fn get_value_delimiter(&self) -> Option<char> {
        self.val_delim
    }

    /// Get the index of this argument, if any
    #[inline]
    pub fn get_index(&self) -> Option<usize> {
        self.index
    }

    /// Get the value hint of this argument
    pub fn get_value_hint(&self) -> ValueHint {
        self.value_hint.unwrap_or_else(|| {
            if self.is_takes_value_set() {
                let type_id = self.get_value_parser().type_id();
                if type_id == crate::parser::AnyValueId::of::<std::path::PathBuf>() {
                    ValueHint::AnyPath
                } else {
                    ValueHint::default()
                }
            } else {
                ValueHint::default()
            }
        })
    }

    /// Get the environment variable name specified for this argument, if any
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::ffi::OsStr;
    /// # use clap::Arg;
    /// let arg = Arg::new("foo").env("ENVIRONMENT");
    /// assert_eq!(Some(OsStr::new("ENVIRONMENT")), arg.get_env());
    /// ```
    #[cfg(feature = "env")]
    pub fn get_env(&self) -> Option<&OsStr> {
        self.env.as_ref().map(|x| x.0)
    }

    /// Get the default values specified for this argument, if any
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// let arg = Arg::new("foo").default_value("default value");
    /// assert_eq!(&["default value"], arg.get_default_values());
    /// ```
    pub fn get_default_values(&self) -> &[&OsStr] {
        &self.default_vals
    }

    /// Checks whether this argument is a positional or not.
    ///
    /// # Examples
    ///
    /// ```
    /// # use clap::Arg;
    /// let arg = Arg::new("foo");
    /// assert_eq!(true, arg.is_positional());
    ///
    /// let arg = Arg::new("foo").long("foo");
    /// assert_eq!(false, arg.is_positional());
    /// ```
    pub fn is_positional(&self) -> bool {
        self.long.is_none() && self.short.is_none()
    }

    /// Reports whether [`Arg::required`] is set
    pub fn is_required_set(&self) -> bool {
        self.is_set(ArgSettings::Required)
    }

    pub(crate) fn is_multiple_values_set(&self) -> bool {
        self.get_num_args().unwrap_or_default().is_multiple()
    }

    pub(crate) fn is_takes_value_set(&self) -> bool {
        self.get_action().takes_values()
    }

    /// Report whether [`Arg::allow_hyphen_values`] is set
    pub fn is_allow_hyphen_values_set(&self) -> bool {
        self.is_set(ArgSettings::AllowHyphenValues)
    }

    /// Behavior when parsing the argument
    pub fn get_action(&self) -> &super::ArgAction {
        const DEFAULT: super::ArgAction = super::ArgAction::Set;
        self.action.as_ref().unwrap_or(&DEFAULT)
    }

    /// Configured parser for argument values
    ///
    /// # Example
    ///
    /// ```rust
    /// let cmd = clap::Command::new("raw")
    ///     .arg(
    ///         clap::Arg::new("port")
    ///             .value_parser(clap::value_parser!(usize))
    ///     );
    /// let value_parser = cmd.get_arguments()
    ///     .find(|a| a.get_id() == "port").unwrap()
    ///     .get_value_parser();
    /// println!("{:?}", value_parser);
    /// ```
    pub fn get_value_parser(&self) -> &super::ValueParser {
        if let Some(value_parser) = self.value_parser.as_ref() {
            value_parser
        } else {
            static DEFAULT: super::ValueParser = super::ValueParser::string();
            &DEFAULT
        }
    }

    /// Report whether [`Arg::global`] is set
    pub fn is_global_set(&self) -> bool {
        self.is_set(ArgSettings::Global)
    }

    /// Report whether [`Arg::next_line_help`] is set
    pub fn is_next_line_help_set(&self) -> bool {
        self.is_set(ArgSettings::NextLineHelp)
    }

    /// Report whether [`Arg::hide`] is set
    pub fn is_hide_set(&self) -> bool {
        self.is_set(ArgSettings::Hidden)
    }

    /// Report whether [`Arg::hide_default_value`] is set
    pub fn is_hide_default_value_set(&self) -> bool {
        self.is_set(ArgSettings::HideDefaultValue)
    }

    /// Report whether [`Arg::hide_possible_values`] is set
    pub fn is_hide_possible_values_set(&self) -> bool {
        self.is_set(ArgSettings::HidePossibleValues)
    }

    /// Report whether [`Arg::hide_env`] is set
    #[cfg(feature = "env")]
    pub fn is_hide_env_set(&self) -> bool {
        self.is_set(ArgSettings::HideEnv)
    }

    /// Report whether [`Arg::hide_env_values`] is set
    #[cfg(feature = "env")]
    pub fn is_hide_env_values_set(&self) -> bool {
        self.is_set(ArgSettings::HideEnvValues)
    }

    /// Report whether [`Arg::hide_short_help`] is set
    pub fn is_hide_short_help_set(&self) -> bool {
        self.is_set(ArgSettings::HiddenShortHelp)
    }

    /// Report whether [`Arg::hide_long_help`] is set
    pub fn is_hide_long_help_set(&self) -> bool {
        self.is_set(ArgSettings::HiddenLongHelp)
    }

    /// Report whether [`Arg::require_equals`] is set
    pub fn is_require_equals_set(&self) -> bool {
        self.is_set(ArgSettings::RequireEquals)
    }

    /// Reports whether [`Arg::exclusive`] is set
    pub fn is_exclusive_set(&self) -> bool {
        self.is_set(ArgSettings::Exclusive)
    }

    /// Reports whether [`Arg::last`] is set
    pub fn is_last_set(&self) -> bool {
        self.is_set(ArgSettings::Last)
    }

    /// Reports whether [`Arg::ignore_case`] is set
    pub fn is_ignore_case_set(&self) -> bool {
        self.is_set(ArgSettings::IgnoreCase)
    }
}

/// # Internally used only
impl<'help> Arg<'help> {
    pub(crate) fn _build(&mut self) {
        if self.action.is_none() {
            if self.num_vals == Some(ValueRange::EMPTY) {
                let action = super::ArgAction::SetTrue;
                self.action = Some(action);
            } else {
                let action =
                    if self.is_positional() && self.num_vals.unwrap_or_default().is_unbounded() {
                        // Allow collecting arguments interleaved with flags
                        //
                        // Bounded values are probably a group and the user should explicitly opt-in to
                        // Append
                        super::ArgAction::Append
                    } else {
                        super::ArgAction::Set
                    };
                self.action = Some(action);
            }
        }
        if let Some(action) = self.action.as_ref() {
            if let Some(default_value) = action.default_value() {
                if self.default_vals.is_empty() {
                    self.default_vals = vec![default_value];
                }
            }
            if let Some(default_value) = action.default_missing_value() {
                if self.default_missing_vals.is_empty() {
                    self.default_missing_vals = vec![default_value];
                }
            }
        }

        if self.value_parser.is_none() {
            if let Some(default) = self.action.as_ref().and_then(|a| a.default_value_parser()) {
                self.value_parser = Some(default);
            } else {
                self.value_parser = Some(super::ValueParser::string());
            }
        }

        let val_names_len = self.val_names.len();
        if val_names_len > 1 {
            self.num_vals.get_or_insert(val_names_len.into());
        } else {
            let nargs = if self.get_action().takes_values() {
                ValueRange::SINGLE
            } else {
                ValueRange::EMPTY
            };
            self.num_vals.get_or_insert(nargs);
        }
    }

    pub(crate) fn longest_filter(&self) -> bool {
        self.is_takes_value_set() || self.long.is_some() || self.short.is_none()
    }

    // Used for positionals when printing
    pub(crate) fn multiple_str(&self) -> &str {
        let mult_vals = self.val_names.len() > 1;
        if (self.is_multiple_values_set() || matches!(*self.get_action(), ArgAction::Append))
            && !mult_vals
        {
            "..."
        } else {
            ""
        }
    }

    // Used for positionals when printing
    pub(crate) fn name_no_brackets(&self) -> Cow<str> {
        debug!("Arg::name_no_brackets:{}", self.name);
        let delim = " ";
        if !self.val_names.is_empty() {
            debug!("Arg::name_no_brackets: val_names={:#?}", self.val_names);

            if self.val_names.len() > 1 {
                Cow::Owned(
                    self.val_names
                        .iter()
                        .map(|n| format!("<{}>", n))
                        .collect::<Vec<_>>()
                        .join(delim),
                )
            } else {
                Cow::Borrowed(self.val_names.first().expect(INTERNAL_ERROR_MSG))
            }
        } else {
            debug!("Arg::name_no_brackets: just name");
            Cow::Borrowed(self.name)
        }
    }

    /// Either multiple values or occurrences
    pub(crate) fn is_multiple(&self) -> bool {
        self.is_multiple_values_set() || matches!(*self.get_action(), ArgAction::Append)
    }

    pub(crate) fn get_display_order(&self) -> usize {
        self.disp_ord.unwrap_or(999)
    }
}

impl<'help> From<&'_ Arg<'help>> for Arg<'help> {
    fn from(a: &Arg<'help>) -> Self {
        a.clone()
    }
}

impl<'help> PartialEq for Arg<'help> {
    fn eq(&self, other: &Arg<'help>) -> bool {
        self.name == other.name
    }
}

impl<'help> PartialOrd for Arg<'help> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'help> Ord for Arg<'help> {
    fn cmp(&self, other: &Arg) -> Ordering {
        self.name.cmp(other.name)
    }
}

impl<'help> Eq for Arg<'help> {}

impl<'help> Display for Arg<'help> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Write the name such --long or -l
        if let Some(l) = self.long {
            write!(f, "--{}", l)?;
        } else if let Some(s) = self.short {
            write!(f, "-{}", s)?;
        }
        let mut need_closing_bracket = false;
        let is_optional_val = self.get_min_vals() == 0;
        if self.is_positional() {
            if is_optional_val {
                let sep = "[";
                need_closing_bracket = true;
                f.write_str(sep)?;
            }
        } else if self.is_takes_value_set() {
            let sep = if self.is_require_equals_set() {
                if is_optional_val {
                    need_closing_bracket = true;
                    "[="
                } else {
                    "="
                }
            } else if is_optional_val {
                need_closing_bracket = true;
                " ["
            } else {
                " "
            };
            f.write_str(sep)?;
        }
        if self.is_takes_value_set() || self.is_positional() {
            let arg_val = render_arg_val(self);
            f.write_str(&arg_val)?;
        } else if matches!(*self.get_action(), ArgAction::Count) {
            f.write_str("...")?;
        }
        if need_closing_bracket {
            f.write_str("]")?;
        }

        Ok(())
    }
}

impl<'help> fmt::Debug for Arg<'help> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        let mut ds = f.debug_struct("Arg");

        #[allow(unused_mut)]
        let mut ds = ds
            .field("id", &self.id)
            .field("name", &self.name)
            .field("help", &self.help)
            .field("long_help", &self.long_help)
            .field("action", &self.action)
            .field("value_parser", &self.value_parser)
            .field("blacklist", &self.blacklist)
            .field("settings", &self.settings)
            .field("overrides", &self.overrides)
            .field("groups", &self.groups)
            .field("requires", &self.requires)
            .field("r_ifs", &self.r_ifs)
            .field("r_unless", &self.r_unless)
            .field("short", &self.short)
            .field("long", &self.long)
            .field("aliases", &self.aliases)
            .field("short_aliases", &self.short_aliases)
            .field("disp_ord", &self.disp_ord)
            .field("val_names", &self.val_names)
            .field("num_vals", &self.num_vals)
            .field("val_delim", &self.val_delim)
            .field("default_vals", &self.default_vals)
            .field("default_vals_ifs", &self.default_vals_ifs)
            .field("terminator", &self.terminator)
            .field("index", &self.index)
            .field("help_heading", &self.help_heading)
            .field("value_hint", &self.value_hint)
            .field("default_missing_vals", &self.default_missing_vals);

        #[cfg(feature = "env")]
        {
            ds = ds.field("env", &self.env);
        }

        ds.finish()
    }
}

/// Write the values such as <name1> <name2>
pub(crate) fn render_arg_val(arg: &Arg) -> String {
    let mut rendered = String::new();

    let delim = " ";

    let arg_name_storage;
    let val_names = if arg.val_names.is_empty() {
        arg_name_storage = [arg.name];
        &arg_name_storage
    } else {
        arg.val_names.as_slice()
    };

    let mut extra_values = false;
    debug_assert!(arg.is_takes_value_set());
    let num_vals = arg.get_num_args().expect(INTERNAL_ERROR_MSG);
    if val_names.len() == 1 {
        let arg_name = format!("<{}>", val_names[0]);
        let min = num_vals.min_values().max(1);
        for n in 1..=min {
            if n != 1 {
                rendered.push_str(delim);
            }
            rendered.push_str(&arg_name);
        }
        extra_values |= min < num_vals.max_values();
    } else {
        debug_assert!(1 < val_names.len());
        for (n, val_name) in val_names.iter().enumerate() {
            let arg_name = format!("<{}>", val_name);
            if n != 0 {
                rendered.push_str(delim);
            }
            rendered.push_str(&arg_name);
        }
        extra_values |= val_names.len() < num_vals.max_values();
    }
    if arg.is_positional() && matches!(*arg.get_action(), ArgAction::Append) {
        extra_values = true;
    }

    if extra_values {
        rendered.push_str("...");
    }

    rendered
}

// Flags
#[cfg(test)]
mod test {
    use super::Arg;
    use super::ArgAction;

    #[test]
    fn flag_display_long() {
        let mut f = Arg::new("flg").long("flag").action(ArgAction::SetTrue);
        f._build();

        assert_eq!(f.to_string(), "--flag");
    }

    #[test]
    fn flag_display_short() {
        let mut f2 = Arg::new("flg").short('f').action(ArgAction::SetTrue);
        f2._build();

        assert_eq!(f2.to_string(), "-f");
    }

    #[test]
    fn flag_display_count() {
        let mut f2 = Arg::new("flg").long("flag").action(ArgAction::Count);
        f2._build();

        assert_eq!(f2.to_string(), "--flag...");
    }

    #[test]
    fn flag_display_single_alias() {
        let mut f = Arg::new("flg")
            .long("flag")
            .visible_alias("als")
            .action(ArgAction::SetTrue);
        f._build();

        assert_eq!(f.to_string(), "--flag")
    }

    #[test]
    fn flag_display_multiple_aliases() {
        let mut f = Arg::new("flg").short('f').action(ArgAction::SetTrue);
        f.aliases = vec![
            ("alias_not_visible", false),
            ("f2", true),
            ("f3", true),
            ("f4", true),
        ];
        f._build();

        assert_eq!(f.to_string(), "-f");
    }

    #[test]
    fn flag_display_single_short_alias() {
        let mut f = Arg::new("flg").short('a').action(ArgAction::SetTrue);
        f.short_aliases = vec![('b', true)];
        f._build();

        assert_eq!(f.to_string(), "-a")
    }

    #[test]
    fn flag_display_multiple_short_aliases() {
        let mut f = Arg::new("flg").short('a').action(ArgAction::SetTrue);
        f.short_aliases = vec![('b', false), ('c', true), ('d', true), ('e', true)];
        f._build();

        assert_eq!(f.to_string(), "-a");
    }

    // Options

    #[test]
    fn option_display_multiple_occurrences() {
        let mut o = Arg::new("opt").long("option").action(ArgAction::Append);
        o._build();

        assert_eq!(o.to_string(), "--option <opt>");
    }

    #[test]
    fn option_display_multiple_values() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .num_args(1..);
        o._build();

        assert_eq!(o.to_string(), "--option <opt>...");
    }

    #[test]
    fn option_display_zero_or_more_values() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .num_args(0..);
        o._build();

        assert_eq!(o.to_string(), "--option [<opt>...]");
    }

    #[test]
    fn option_display_one_or_more_values() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .num_args(1..);
        o._build();

        assert_eq!(o.to_string(), "--option <opt>...");
    }

    #[test]
    fn option_display_zero_or_more_values_with_value_name() {
        let mut o = Arg::new("opt")
            .short('o')
            .action(ArgAction::Set)
            .num_args(0..)
            .value_names(&["file"]);
        o._build();

        assert_eq!(o.to_string(), "-o [<file>...]");
    }

    #[test]
    fn option_display_one_or_more_values_with_value_name() {
        let mut o = Arg::new("opt")
            .short('o')
            .action(ArgAction::Set)
            .num_args(1..)
            .value_names(&["file"]);
        o._build();

        assert_eq!(o.to_string(), "-o <file>...");
    }

    #[test]
    fn option_display_optional_value() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .num_args(0..=1);
        o._build();

        assert_eq!(o.to_string(), "--option [<opt>]");
    }

    #[test]
    fn option_display_value_names() {
        let mut o = Arg::new("opt")
            .short('o')
            .action(ArgAction::Set)
            .value_names(&["file", "name"]);
        o._build();

        assert_eq!(o.to_string(), "-o <file> <name>");
    }

    #[test]
    fn option_display3() {
        let mut o = Arg::new("opt")
            .short('o')
            .num_args(1..)
            .action(ArgAction::Set)
            .value_names(&["file", "name"]);
        o._build();

        assert_eq!(o.to_string(), "-o <file> <name>...");
    }

    #[test]
    fn option_display_single_alias() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .visible_alias("als");
        o._build();

        assert_eq!(o.to_string(), "--option <opt>");
    }

    #[test]
    fn option_display_multiple_aliases() {
        let mut o = Arg::new("opt")
            .long("option")
            .action(ArgAction::Set)
            .visible_aliases(&["als2", "als3", "als4"])
            .alias("als_not_visible");
        o._build();

        assert_eq!(o.to_string(), "--option <opt>");
    }

    #[test]
    fn option_display_single_short_alias() {
        let mut o = Arg::new("opt")
            .short('a')
            .action(ArgAction::Set)
            .visible_short_alias('b');
        o._build();

        assert_eq!(o.to_string(), "-a <opt>");
    }

    #[test]
    fn option_display_multiple_short_aliases() {
        let mut o = Arg::new("opt")
            .short('a')
            .action(ArgAction::Set)
            .visible_short_aliases(&['b', 'c', 'd'])
            .short_alias('e');
        o._build();

        assert_eq!(o.to_string(), "-a <opt>");
    }

    // Positionals

    #[test]
    fn positional_display_multiple_values() {
        let mut p = Arg::new("pos").index(1).num_args(1..);
        p._build();

        assert_eq!(p.to_string(), "<pos>...");
    }

    #[test]
    fn positional_display_zero_or_more_values() {
        let mut p = Arg::new("pos").index(1).num_args(0..);
        p._build();

        assert_eq!(p.to_string(), "[<pos>...]");
    }

    #[test]
    fn positional_display_one_or_more_values() {
        let mut p = Arg::new("pos").index(1).num_args(1..);
        p._build();

        assert_eq!(p.to_string(), "<pos>...");
    }

    #[test]
    fn positional_display_optional_value() {
        let mut p = Arg::new("pos")
            .index(1)
            .num_args(0..=1)
            .action(ArgAction::Set);
        p._build();

        assert_eq!(p.to_string(), "[<pos>]");
    }

    #[test]
    fn positional_display_multiple_occurrences() {
        let mut p = Arg::new("pos").index(1).action(ArgAction::Append);
        p._build();

        assert_eq!(p.to_string(), "<pos>...");
    }

    #[test]
    fn positional_display_required() {
        let mut p = Arg::new("pos").index(1).required(true);
        p._build();

        assert_eq!(p.to_string(), "<pos>");
    }

    #[test]
    fn positional_display_val_names() {
        let mut p = Arg::new("pos").index(1).value_names(&["file1", "file2"]);
        p._build();

        assert_eq!(p.to_string(), "<file1> <file2>");
    }

    #[test]
    fn positional_display_val_names_req() {
        let mut p = Arg::new("pos")
            .index(1)
            .required(true)
            .value_names(&["file1", "file2"]);
        p._build();

        assert_eq!(p.to_string(), "<file1> <file2>");
    }
}
