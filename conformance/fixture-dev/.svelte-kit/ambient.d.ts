
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
	export const STM32_PRG_PATH: string;
	export const STARSHIP_SHELL: string;
	export const MANPATH: string;
	export const __MISE_DIFF: string;
	export const NODE: string;
	export const FNM_LOGLEVEL: string;
	export const LESS_TERMCAP_md: string;
	export const INIT_CWD: string;
	export const HI_DPI_MODE: string;
	export const ANDROID_HOME: string;
	export const TERM: string;
	export const SHELL: string;
	export const SAVEHIST: string;
	export const FNM_NODE_DIST_MIRROR: string;
	export const HISTSIZE: string;
	export const HOMEBREW_REPOSITORY: string;
	export const TMPDIR: string;
	export const npm_config_global_prefix: string;
	export const LIBRARY_PATH: string;
	export const FPATH: string;
	export const WINDOWID: string;
	export const OLDPWD: string;
	export const COLOR: string;
	export const npm_config_noproxy: string;
	export const npm_config_registry: string;
	export const npm_config_local_prefix: string;
	export const LC_ALL: string;
	export const FNM_COREPACK_ENABLED: string;
	export const ZSH_TMUX_TERM: string;
	export const USER: string;
	export const LS_COLORS: string;
	export const COMMAND_MODE: string;
	export const npm_config_globalconfig: string;
	export const _ZSH_TMUX_FIXED_CONFIG: string;
	export const FZF_BASE: string;
	export const SSH_AUTH_SOCK: string;
	export const __CF_USER_TEXT_ENCODING: string;
	export const HOMEBREW_NO_AUTO_UPDATE: string;
	export const npm_execpath: string;
	export const SKIM_DEFAULT_COMMAND: string;
	export const PAGER: string;
	export const ZSH_TMUX_CONFIG: string;
	export const FNM_VERSION_FILE_STRATEGY: string;
	export const LSCOLORS: string;
	export const FNM_ARCH: string;
	export const PATH: string;
	export const SRC_DIR: string;
	export const npm_package_json: string;
	export const WORK_DIR: string;
	export const npm_config_userconfig: string;
	export const npm_config_init_module: string;
	export const __CFBundleIdentifier: string;
	export const npm_command: string;
	export const PWD: string;
	export const FZF_ALT_C_COMMAND: string;
	export const FLOAT_VIEW_FIXTURE_PORT: string;
	export const npm_lifecycle_event: string;
	export const EDITOR: string;
	export const KITTY_PID: string;
	export const npm_package_name: string;
	export const LANG: string;
	export const npm_config_npm_version: string;
	export const FNM_MULTISHELL_PATH: string;
	export const XPC_FLAGS: string;
	export const npm_package_engines_node: string;
	export const npm_config_node_gyp: string;
	export const XPC_SERVICE_NAME: string;
	export const GPG_TTY: string;
	export const SHLVL: string;
	export const MANPAGER: string;
	export const HOME: string;
	export const TERMINFO: string;
	export const __MISE_ORIG_PATH: string;
	export const ATLASSIAN_MCP_TOKEN: string;
	export const HOMEBREW_PREFIX: string;
	export const FNM_DIR: string;
	export const MISE_SHELL: string;
	export const GITHUB_MCP_TOKEN: string;
	export const npm_config_cache: string;
	export const __MISE_ZSH_CHPWD_RAN: string;
	export const STARSHIP_SESSION_KEY: string;
	export const LOGNAME: string;
	export const LESS: string;
	export const npm_lifecycle_script: string;
	export const FZF_CTRL_T_COMMAND: string;
	export const FZF_DEFAULT_COMMAND: string;
	export const GOPATH: string;
	export const BUN_INSTALL: string;
	export const npm_config_user_agent: string;
	export const KITTY_INSTALLATION_DIR: string;
	export const FNM_RESOLVE_ENGINES: string;
	export const KITTY_WINDOW_ID: string;
	export const TERMINAL: string;
	export const __MISE_SESSION: string;
	export const HOMEBREW_CELLAR: string;
	export const INFOPATH: string;
	export const DISPLAY: string;
	export const __MISE_ZSH_PRECMD_RUN: string;
	export const HISTFILE: string;
	export const npm_node_execpath: string;
	export const npm_config_prefix: string;
	export const KITTY_PUBLIC_KEY: string;
	export const COLORTERM: string;
	export const _: string;
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
		STM32_PRG_PATH: string;
		STARSHIP_SHELL: string;
		MANPATH: string;
		__MISE_DIFF: string;
		NODE: string;
		FNM_LOGLEVEL: string;
		LESS_TERMCAP_md: string;
		INIT_CWD: string;
		HI_DPI_MODE: string;
		ANDROID_HOME: string;
		TERM: string;
		SHELL: string;
		SAVEHIST: string;
		FNM_NODE_DIST_MIRROR: string;
		HISTSIZE: string;
		HOMEBREW_REPOSITORY: string;
		TMPDIR: string;
		npm_config_global_prefix: string;
		LIBRARY_PATH: string;
		FPATH: string;
		WINDOWID: string;
		OLDPWD: string;
		COLOR: string;
		npm_config_noproxy: string;
		npm_config_registry: string;
		npm_config_local_prefix: string;
		LC_ALL: string;
		FNM_COREPACK_ENABLED: string;
		ZSH_TMUX_TERM: string;
		USER: string;
		LS_COLORS: string;
		COMMAND_MODE: string;
		npm_config_globalconfig: string;
		_ZSH_TMUX_FIXED_CONFIG: string;
		FZF_BASE: string;
		SSH_AUTH_SOCK: string;
		__CF_USER_TEXT_ENCODING: string;
		HOMEBREW_NO_AUTO_UPDATE: string;
		npm_execpath: string;
		SKIM_DEFAULT_COMMAND: string;
		PAGER: string;
		ZSH_TMUX_CONFIG: string;
		FNM_VERSION_FILE_STRATEGY: string;
		LSCOLORS: string;
		FNM_ARCH: string;
		PATH: string;
		SRC_DIR: string;
		npm_package_json: string;
		WORK_DIR: string;
		npm_config_userconfig: string;
		npm_config_init_module: string;
		__CFBundleIdentifier: string;
		npm_command: string;
		PWD: string;
		FZF_ALT_C_COMMAND: string;
		FLOAT_VIEW_FIXTURE_PORT: string;
		npm_lifecycle_event: string;
		EDITOR: string;
		KITTY_PID: string;
		npm_package_name: string;
		LANG: string;
		npm_config_npm_version: string;
		FNM_MULTISHELL_PATH: string;
		XPC_FLAGS: string;
		npm_package_engines_node: string;
		npm_config_node_gyp: string;
		XPC_SERVICE_NAME: string;
		GPG_TTY: string;
		SHLVL: string;
		MANPAGER: string;
		HOME: string;
		TERMINFO: string;
		__MISE_ORIG_PATH: string;
		ATLASSIAN_MCP_TOKEN: string;
		HOMEBREW_PREFIX: string;
		FNM_DIR: string;
		MISE_SHELL: string;
		GITHUB_MCP_TOKEN: string;
		npm_config_cache: string;
		__MISE_ZSH_CHPWD_RAN: string;
		STARSHIP_SESSION_KEY: string;
		LOGNAME: string;
		LESS: string;
		npm_lifecycle_script: string;
		FZF_CTRL_T_COMMAND: string;
		FZF_DEFAULT_COMMAND: string;
		GOPATH: string;
		BUN_INSTALL: string;
		npm_config_user_agent: string;
		KITTY_INSTALLATION_DIR: string;
		FNM_RESOLVE_ENGINES: string;
		KITTY_WINDOW_ID: string;
		TERMINAL: string;
		__MISE_SESSION: string;
		HOMEBREW_CELLAR: string;
		INFOPATH: string;
		DISPLAY: string;
		__MISE_ZSH_PRECMD_RUN: string;
		HISTFILE: string;
		npm_node_execpath: string;
		npm_config_prefix: string;
		KITTY_PUBLIC_KEY: string;
		COLORTERM: string;
		_: string;
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
