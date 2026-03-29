
// this file is generated — do not edit it


/// <reference types="@sveltejs/kit" />

/**
 * This module provides access to environment variables that are injected _statically_ into your bundle at build time and are limited to _private_ access.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Static environment variables are [loaded by Vite](https://vitejs.dev/guide/env-and-mode.html#env-files) from `.env` files and `process.env` at build time and then statically injected into your bundle at build time, enabling optimisations like dead code elimination.
 * 
 * **_Private_ access:**
 * 
 * - This module cannot be imported into client-side code
 * - This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured)
 * 
 * For example, given the following build time environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://site.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { ENVIRONMENT, PUBLIC_BASE_URL } from '$env/static/private';
 * 
 * console.log(ENVIRONMENT); // => "production"
 * console.log(PUBLIC_BASE_URL); // => throws error during build
 * ```
 * 
 * The above values will be the same _even if_ different values for `ENVIRONMENT` or `PUBLIC_BASE_URL` are set at runtime, as they are statically replaced in your code with their build time values.
 */
declare module '$env/static/private' {
	export const SHELL: string;
	export const npm_command: string;
	export const COREPACK_ENABLE_AUTO_PIN: string;
	export const PKG_CONFIG_FOR_TARGET: string;
	export const __ETC_PROFILE_DONE: string;
	export const OBJDUMP_FOR_TARGET: string;
	export const npm_config_userconfig: string;
	export const COLORTERM: string;
	export const XDG_CONFIG_DIRS: string;
	export const npm_config_cache: string;
	export const NIX_BUILD_CORES: string;
	export const NIX_GCROOT: string;
	export const configureFlags: string;
	export const mesonFlags: string;
	export const LAST_EXIT_CODE: string;
	export const shell: string;
	export const SIZE_FOR_TARGET: string;
	export const depsHostHost: string;
	export const NODE: string;
	export const AS_FOR_TARGET: string;
	export const SSH_AUTH_SOCK: string;
	export const CC_FOR_TARGET: string;
	export const STRINGS: string;
	export const depsTargetTarget: string;
	export const LD_FOR_TARGET: string;
	export const XCURSOR_PATH: string;
	export const MEMORY_PRESSURE_WRITE: string;
	export const stdenv: string;
	export const PROMPT_MULTILINE_INDICATOR: string;
	export const COLOR: string;
	export const LOCALE_ARCHIVE_2_27: string;
	export const npm_config_local_prefix: string;
	export const PKG_CONFIG_PATH_FOR_TARGET: string;
	export const builder: string;
	export const XMODIFIERS: string;
	export const PROMPT_INDICATOR_VI_INSERT: string;
	export const KITTY_PID: string;
	export const shellHook: string;
	export const NO_AT_BRIDGE: string;
	export const npm_config_globalconfig: string;
	export const NIX_BINTOOLS_FOR_TARGET: string;
	export const XCURSOR_SIZE: string;
	export const CLAUDE_CODE_MAX_OUTPUT_TOKENS: string;
	export const NIX_LDFLAGS_FOR_TARGET: string;
	export const EDITOR: string;
	export const phases: string;
	export const XDG_SEAT: string;
	export const PWD: string;
	export const NIX_PROFILES: string;
	export const SOURCE_DATE_EPOCH: string;
	export const LOGNAME: string;
	export const ATUIN_SHLVL: string;
	export const XDG_SESSION_TYPE: string;
	export const NIX_ENFORCE_NO_NATIVE: string;
	export const NIX_BINTOOLS_WRAPPER_TARGET_TARGET_x86_64_unknown_linux_gnu: string;
	export const CUPS_DATADIR: string;
	export const NIX_PATH: string;
	export const npm_config_init_module: string;
	export const STRIP_FOR_TARGET: string;
	export const SYSTEMD_EXEC_PID: string;
	export const NIXPKGS_CONFIG: string;
	export const RANLIB_FOR_TARGET: string;
	export const CXX: string;
	export const _: string;
	export const DESKTOP_STARTUP_ID: string;
	export const CMD_DURATION_MS: string;
	export const TEMPDIR: string;
	export const system: string;
	export const KITTY_PUBLIC_KEY: string;
	export const NoDefaultCurrentDirectoryInExePath: string;
	export const TRANSIENT_PROMPT_MULTILINE_INDICATOR: string;
	export const STRINGS_FOR_TARGET: string;
	export const HOST_PATH: string;
	export const NIX_PKG_CONFIG_WRAPPER_TARGET_TARGET_x86_64_unknown_linux_gnu: string;
	export const SQLX_OFFLINE: string;
	export const CLAUDECODE: string;
	export const IN_NIX_SHELL: string;
	export const doInstallCheck: string;
	export const HOME: string;
	export const NIX_BINTOOLS: string;
	export const SSH_ASKPASS: string;
	export const LANG: string;
	export const NIXOS_OZONE_WL: string;
	export const XDG_CURRENT_DESKTOP: string;
	export const depsTargetTargetPropagated: string;
	export const npm_package_version: string;
	export const MEMORY_PRESSURE_WATCH: string;
	export const STARSHIP_SHELL: string;
	export const WAYLAND_DISPLAY: string;
	export const cmakeFlags: string;
	export const outputs: string;
	export const GIO_EXTRA_MODULES: string;
	export const KITTY_WINDOW_ID: string;
	export const NIX_STORE: string;
	export const TMPDIR: string;
	export const NIX_CFLAGS_COMPILE_FOR_TARGET: string;
	export const https_proxy: string;
	export const READELF_FOR_TARGET: string;
	export const LD: string;
	export const INVOCATION_ID: string;
	export const buildPhase: string;
	export const NIRI_SOCKET: string;
	export const AR_FOR_TARGET: string;
	export const MANAGERPID: string;
	export const INIT_CWD: string;
	export const READELF: string;
	export const GTK_A11Y: string;
	export const STARSHIP_SESSION_KEY: string;
	export const QT_QPA_PLATFORM: string;
	export const NIX_USER_PROFILE_DIR: string;
	export const INFOPATH: string;
	export const npm_lifecycle_script: string;
	export const doCheck: string;
	export const npm_config_npm_version: string;
	export const depsBuildBuild: string;
	export const TERMINFO: string;
	export const TERM: string;
	export const npm_package_name: string;
	export const DISABLE_INSTALLATION_CHECKS: string;
	export const PROMPT_INDICATOR_VI_NORMAL: string;
	export const GTK_PATH: string;
	export const SIZE: string;
	export const propagatedNativeBuildInputs: string;
	export const npm_config_prefix: string;
	export const strictDeps: string;
	export const USER: string;
	export const NIX_CC_WRAPPER_TARGET_TARGET_x86_64_unknown_linux_gnu: string;
	export const PROMPT_INDICATOR: string;
	export const TZDIR: string;
	export const AR: string;
	export const AS: string;
	export const VISUAL: string;
	export const TEMP: string;
	export const NIX_BINTOOLS_WRAPPER_TARGET_HOST_x86_64_unknown_linux_gnu: string;
	export const DISPLAY: string;
	export const npm_lifecycle_event: string;
	export const SHLVL: string;
	export const NIX_BUILD_TOP: string;
	export const CXX_FOR_TARGET: string;
	export const NM: string;
	export const GIT_EDITOR: string;
	export const PAGER: string;
	export const NIX_CFLAGS_COMPILE: string;
	export const QTWEBKIT_PLUGIN_PATH: string;
	export const patches: string;
	export const __NIXOS_SET_ENVIRONMENT_DONE: string;
	export const XDG_VTNR: string;
	export const buildInputs: string;
	export const XDG_SESSION_ID: string;
	export const TRANSIENT_PROMPT_COMMAND_RIGHT: string;
	export const preferLocalBuild: string;
	export const http_proxy: string;
	export const LOCALE_ARCHIVE: string;
	export const ATUIN_SESSION: string;
	export const MANAGERPIDFDID: string;
	export const LESSKEYIN_SYSTEM: string;
	export const npm_config_user_agent: string;
	export const TERMINFO_DIRS: string;
	export const OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE: string;
	export const npm_execpath: string;
	export const LD_LIBRARY_PATH: string;
	export const ATUIN_HISTORY_ID: string;
	export const DISABLE_AUTOUPDATER: string;
	export const XDG_RUNTIME_DIR: string;
	export const NM_FOR_TARGET: string;
	export const OBJCOPY_FOR_TARGET: string;
	export const depsBuildTarget: string;
	export const CLAUDE_CODE_ENTRYPOINT: string;
	export const OBJCOPY: string;
	export const NIX_XDG_DESKTOP_PORTAL_DIR: string;
	export const out: string;
	export const npm_package_json: string;
	export const NU_VERSION: string;
	export const STRIP: string;
	export const JOURNAL_STREAM: string;
	export const XCURSOR_THEME: string;
	export const XDG_DATA_DIRS: string;
	export const GDK_BACKEND: string;
	export const LIBEXEC_PATH: string;
	export const TMP: string;
	export const OBJDUMP: string;
	export const npm_config_noproxy: string;
	export const PATH: string;
	export const propagatedBuildInputs: string;
	export const npm_config_node_gyp: string;
	export const dontAddDisableDepTrack: string;
	export const CC: string;
	export const NIX_CC_FOR_TARGET: string;
	export const NIX_CC: string;
	export const FILE_PWD: string;
	export const DBUS_SESSION_BUS_ADDRESS: string;
	export const depsBuildTargetPropagated: string;
	export const depsBuildBuildPropagated: string;
	export const npm_config_global_prefix: string;
	export const NIX_CC_WRAPPER_TARGET_HOST_x86_64_unknown_linux_gnu: string;
	export const QT_PLUGIN_PATH: string;
	export const CONFIG_SHELL: string;
	export const CURRENT_FILE: string;
	export const KITTY_INSTALLATION_DIR: string;
	export const __structuredAttrs: string;
	export const npm_node_execpath: string;
	export const RANLIB: string;
	export const NIX_HARDENING_ENABLE: string;
	export const OLDPWD: string;
	export const NIX_LDFLAGS: string;
	export const FORCE_AUTOUPDATE_PLUGINS: string;
	export const nativeBuildInputs: string;
	export const name: string;
	export const depsHostHostPropagated: string;
	export const NODE_ENV: string;
}

/**
 * This module provides access to environment variables that are injected _statically_ into your bundle at build time and are _publicly_ accessible.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Static environment variables are [loaded by Vite](https://vitejs.dev/guide/env-and-mode.html#env-files) from `.env` files and `process.env` at build time and then statically injected into your bundle at build time, enabling optimisations like dead code elimination.
 * 
 * **_Public_ access:**
 * 
 * - This module _can_ be imported into client-side code
 * - **Only** variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`) are included
 * 
 * For example, given the following build time environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://site.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { ENVIRONMENT, PUBLIC_BASE_URL } from '$env/static/public';
 * 
 * console.log(ENVIRONMENT); // => throws error during build
 * console.log(PUBLIC_BASE_URL); // => "http://site.com"
 * ```
 * 
 * The above values will be the same _even if_ different values for `ENVIRONMENT` or `PUBLIC_BASE_URL` are set at runtime, as they are statically replaced in your code with their build time values.
 */
declare module '$env/static/public' {
	
}

/**
 * This module provides access to environment variables set _dynamically_ at runtime and that are limited to _private_ access.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Dynamic environment variables are defined by the platform you're running on. For example if you're using [`adapter-node`](https://github.com/sveltejs/kit/tree/main/packages/adapter-node) (or running [`vite preview`](https://svelte.dev/docs/kit/cli)), this is equivalent to `process.env`.
 * 
 * **_Private_ access:**
 * 
 * - This module cannot be imported into client-side code
 * - This module includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured)
 * 
 * > [!NOTE] In `dev`, `$env/dynamic` includes environment variables from `.env`. In `prod`, this behavior will depend on your adapter.
 * 
 * > [!NOTE] To get correct types, environment variables referenced in your code should be declared (for example in an `.env` file), even if they don't have a value until the app is deployed:
 * >
 * > ```env
 * > MY_FEATURE_FLAG=
 * > ```
 * >
 * > You can override `.env` values from the command line like so:
 * >
 * > ```sh
 * > MY_FEATURE_FLAG="enabled" npm run dev
 * > ```
 * 
 * For example, given the following runtime environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://site.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { env } from '$env/dynamic/private';
 * 
 * console.log(env.ENVIRONMENT); // => "production"
 * console.log(env.PUBLIC_BASE_URL); // => undefined
 * ```
 */
declare module '$env/dynamic/private' {
	export const env: {
		SHELL: string;
		npm_command: string;
		COREPACK_ENABLE_AUTO_PIN: string;
		PKG_CONFIG_FOR_TARGET: string;
		__ETC_PROFILE_DONE: string;
		OBJDUMP_FOR_TARGET: string;
		npm_config_userconfig: string;
		COLORTERM: string;
		XDG_CONFIG_DIRS: string;
		npm_config_cache: string;
		NIX_BUILD_CORES: string;
		NIX_GCROOT: string;
		configureFlags: string;
		mesonFlags: string;
		LAST_EXIT_CODE: string;
		shell: string;
		SIZE_FOR_TARGET: string;
		depsHostHost: string;
		NODE: string;
		AS_FOR_TARGET: string;
		SSH_AUTH_SOCK: string;
		CC_FOR_TARGET: string;
		STRINGS: string;
		depsTargetTarget: string;
		LD_FOR_TARGET: string;
		XCURSOR_PATH: string;
		MEMORY_PRESSURE_WRITE: string;
		stdenv: string;
		PROMPT_MULTILINE_INDICATOR: string;
		COLOR: string;
		LOCALE_ARCHIVE_2_27: string;
		npm_config_local_prefix: string;
		PKG_CONFIG_PATH_FOR_TARGET: string;
		builder: string;
		XMODIFIERS: string;
		PROMPT_INDICATOR_VI_INSERT: string;
		KITTY_PID: string;
		shellHook: string;
		NO_AT_BRIDGE: string;
		npm_config_globalconfig: string;
		NIX_BINTOOLS_FOR_TARGET: string;
		XCURSOR_SIZE: string;
		CLAUDE_CODE_MAX_OUTPUT_TOKENS: string;
		NIX_LDFLAGS_FOR_TARGET: string;
		EDITOR: string;
		phases: string;
		XDG_SEAT: string;
		PWD: string;
		NIX_PROFILES: string;
		SOURCE_DATE_EPOCH: string;
		LOGNAME: string;
		ATUIN_SHLVL: string;
		XDG_SESSION_TYPE: string;
		NIX_ENFORCE_NO_NATIVE: string;
		NIX_BINTOOLS_WRAPPER_TARGET_TARGET_x86_64_unknown_linux_gnu: string;
		CUPS_DATADIR: string;
		NIX_PATH: string;
		npm_config_init_module: string;
		STRIP_FOR_TARGET: string;
		SYSTEMD_EXEC_PID: string;
		NIXPKGS_CONFIG: string;
		RANLIB_FOR_TARGET: string;
		CXX: string;
		_: string;
		DESKTOP_STARTUP_ID: string;
		CMD_DURATION_MS: string;
		TEMPDIR: string;
		system: string;
		KITTY_PUBLIC_KEY: string;
		NoDefaultCurrentDirectoryInExePath: string;
		TRANSIENT_PROMPT_MULTILINE_INDICATOR: string;
		STRINGS_FOR_TARGET: string;
		HOST_PATH: string;
		NIX_PKG_CONFIG_WRAPPER_TARGET_TARGET_x86_64_unknown_linux_gnu: string;
		SQLX_OFFLINE: string;
		CLAUDECODE: string;
		IN_NIX_SHELL: string;
		doInstallCheck: string;
		HOME: string;
		NIX_BINTOOLS: string;
		SSH_ASKPASS: string;
		LANG: string;
		NIXOS_OZONE_WL: string;
		XDG_CURRENT_DESKTOP: string;
		depsTargetTargetPropagated: string;
		npm_package_version: string;
		MEMORY_PRESSURE_WATCH: string;
		STARSHIP_SHELL: string;
		WAYLAND_DISPLAY: string;
		cmakeFlags: string;
		outputs: string;
		GIO_EXTRA_MODULES: string;
		KITTY_WINDOW_ID: string;
		NIX_STORE: string;
		TMPDIR: string;
		NIX_CFLAGS_COMPILE_FOR_TARGET: string;
		https_proxy: string;
		READELF_FOR_TARGET: string;
		LD: string;
		INVOCATION_ID: string;
		buildPhase: string;
		NIRI_SOCKET: string;
		AR_FOR_TARGET: string;
		MANAGERPID: string;
		INIT_CWD: string;
		READELF: string;
		GTK_A11Y: string;
		STARSHIP_SESSION_KEY: string;
		QT_QPA_PLATFORM: string;
		NIX_USER_PROFILE_DIR: string;
		INFOPATH: string;
		npm_lifecycle_script: string;
		doCheck: string;
		npm_config_npm_version: string;
		depsBuildBuild: string;
		TERMINFO: string;
		TERM: string;
		npm_package_name: string;
		DISABLE_INSTALLATION_CHECKS: string;
		PROMPT_INDICATOR_VI_NORMAL: string;
		GTK_PATH: string;
		SIZE: string;
		propagatedNativeBuildInputs: string;
		npm_config_prefix: string;
		strictDeps: string;
		USER: string;
		NIX_CC_WRAPPER_TARGET_TARGET_x86_64_unknown_linux_gnu: string;
		PROMPT_INDICATOR: string;
		TZDIR: string;
		AR: string;
		AS: string;
		VISUAL: string;
		TEMP: string;
		NIX_BINTOOLS_WRAPPER_TARGET_HOST_x86_64_unknown_linux_gnu: string;
		DISPLAY: string;
		npm_lifecycle_event: string;
		SHLVL: string;
		NIX_BUILD_TOP: string;
		CXX_FOR_TARGET: string;
		NM: string;
		GIT_EDITOR: string;
		PAGER: string;
		NIX_CFLAGS_COMPILE: string;
		QTWEBKIT_PLUGIN_PATH: string;
		patches: string;
		__NIXOS_SET_ENVIRONMENT_DONE: string;
		XDG_VTNR: string;
		buildInputs: string;
		XDG_SESSION_ID: string;
		TRANSIENT_PROMPT_COMMAND_RIGHT: string;
		preferLocalBuild: string;
		http_proxy: string;
		LOCALE_ARCHIVE: string;
		ATUIN_SESSION: string;
		MANAGERPIDFDID: string;
		LESSKEYIN_SYSTEM: string;
		npm_config_user_agent: string;
		TERMINFO_DIRS: string;
		OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE: string;
		npm_execpath: string;
		LD_LIBRARY_PATH: string;
		ATUIN_HISTORY_ID: string;
		DISABLE_AUTOUPDATER: string;
		XDG_RUNTIME_DIR: string;
		NM_FOR_TARGET: string;
		OBJCOPY_FOR_TARGET: string;
		depsBuildTarget: string;
		CLAUDE_CODE_ENTRYPOINT: string;
		OBJCOPY: string;
		NIX_XDG_DESKTOP_PORTAL_DIR: string;
		out: string;
		npm_package_json: string;
		NU_VERSION: string;
		STRIP: string;
		JOURNAL_STREAM: string;
		XCURSOR_THEME: string;
		XDG_DATA_DIRS: string;
		GDK_BACKEND: string;
		LIBEXEC_PATH: string;
		TMP: string;
		OBJDUMP: string;
		npm_config_noproxy: string;
		PATH: string;
		propagatedBuildInputs: string;
		npm_config_node_gyp: string;
		dontAddDisableDepTrack: string;
		CC: string;
		NIX_CC_FOR_TARGET: string;
		NIX_CC: string;
		FILE_PWD: string;
		DBUS_SESSION_BUS_ADDRESS: string;
		depsBuildTargetPropagated: string;
		depsBuildBuildPropagated: string;
		npm_config_global_prefix: string;
		NIX_CC_WRAPPER_TARGET_HOST_x86_64_unknown_linux_gnu: string;
		QT_PLUGIN_PATH: string;
		CONFIG_SHELL: string;
		CURRENT_FILE: string;
		KITTY_INSTALLATION_DIR: string;
		__structuredAttrs: string;
		npm_node_execpath: string;
		RANLIB: string;
		NIX_HARDENING_ENABLE: string;
		OLDPWD: string;
		NIX_LDFLAGS: string;
		FORCE_AUTOUPDATE_PLUGINS: string;
		nativeBuildInputs: string;
		name: string;
		depsHostHostPropagated: string;
		NODE_ENV: string;
		[key: `PUBLIC_${string}`]: undefined;
		[key: `${string}`]: string | undefined;
	}
}

/**
 * This module provides access to environment variables set _dynamically_ at runtime and that are _publicly_ accessible.
 * 
 * |         | Runtime                                                                    | Build time                                                               |
 * | ------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
 * | Private | [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private) | [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private) |
 * | Public  | [`$env/dynamic/public`](https://svelte.dev/docs/kit/$env-dynamic-public)   | [`$env/static/public`](https://svelte.dev/docs/kit/$env-static-public)   |
 * 
 * Dynamic environment variables are defined by the platform you're running on. For example if you're using [`adapter-node`](https://github.com/sveltejs/kit/tree/main/packages/adapter-node) (or running [`vite preview`](https://svelte.dev/docs/kit/cli)), this is equivalent to `process.env`.
 * 
 * **_Public_ access:**
 * 
 * - This module _can_ be imported into client-side code
 * - **Only** variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`) are included
 * 
 * > [!NOTE] In `dev`, `$env/dynamic` includes environment variables from `.env`. In `prod`, this behavior will depend on your adapter.
 * 
 * > [!NOTE] To get correct types, environment variables referenced in your code should be declared (for example in an `.env` file), even if they don't have a value until the app is deployed:
 * >
 * > ```env
 * > MY_FEATURE_FLAG=
 * > ```
 * >
 * > You can override `.env` values from the command line like so:
 * >
 * > ```sh
 * > MY_FEATURE_FLAG="enabled" npm run dev
 * > ```
 * 
 * For example, given the following runtime environment:
 * 
 * ```env
 * ENVIRONMENT=production
 * PUBLIC_BASE_URL=http://example.com
 * ```
 * 
 * With the default `publicPrefix` and `privatePrefix`:
 * 
 * ```ts
 * import { env } from '$env/dynamic/public';
 * console.log(env.ENVIRONMENT); // => undefined, not public
 * console.log(env.PUBLIC_BASE_URL); // => "http://example.com"
 * ```
 * 
 * ```
 * 
 * ```
 */
declare module '$env/dynamic/public' {
	export const env: {
		[key: `PUBLIC_${string}`]: string | undefined;
	}
}
