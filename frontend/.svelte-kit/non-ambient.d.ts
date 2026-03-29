
// this file is generated — do not edit it


declare module "svelte/elements" {
	export interface HTMLAttributes<T> {
		'data-sveltekit-keepfocus'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-noscroll'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-preload-code'?:
			| true
			| ''
			| 'eager'
			| 'viewport'
			| 'hover'
			| 'tap'
			| 'off'
			| undefined
			| null;
		'data-sveltekit-preload-data'?: true | '' | 'hover' | 'tap' | 'off' | undefined | null;
		'data-sveltekit-reload'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-replacestate'?: true | '' | 'off' | undefined | null;
	}
}

export {};


declare module "$app/types" {
	type MatcherParam<M> = M extends (param : string) => param is (infer U extends string) ? U : string;

	export interface AppTypes {
		RouteId(): "/(wide)" | "/(home)" | "/(fullwidth)" | "/(default)" | "/(article)" | "/" | "/(default)/about" | "/(article)/article" | "/(default)/book-edition" | "/(default)/books" | "/(wide)/book" | "/(default)/drafts" | "/(default)/forks" | "/(fullwidth)/graph" | "/(default)/guide" | "/(fullwidth)/library" | "/(default)/new-question" | "/(default)/new-series" | "/(default)/new" | "/(default)/notifications" | "/(default)/profile" | "/(default)/questions" | "/(default)/question" | "/(default)/roadmap" | "/(default)/series" | "/(default)/settings" | "/(default)/skill-trees" | "/(default)/skill-tree" | "/(default)/skill-tree/new" | "/(fullwidth)/skills" | "/(default)/tag";
		RouteParams(): {
			
		};
		LayoutParams(): {
			"/(wide)": Record<string, never>;
			"/(home)": Record<string, never>;
			"/(fullwidth)": Record<string, never>;
			"/(default)": Record<string, never>;
			"/(article)": Record<string, never>;
			"/": Record<string, never>;
			"/(default)/about": Record<string, never>;
			"/(article)/article": Record<string, never>;
			"/(default)/book-edition": Record<string, never>;
			"/(default)/books": Record<string, never>;
			"/(wide)/book": Record<string, never>;
			"/(default)/drafts": Record<string, never>;
			"/(default)/forks": Record<string, never>;
			"/(fullwidth)/graph": Record<string, never>;
			"/(default)/guide": Record<string, never>;
			"/(fullwidth)/library": Record<string, never>;
			"/(default)/new-question": Record<string, never>;
			"/(default)/new-series": Record<string, never>;
			"/(default)/new": Record<string, never>;
			"/(default)/notifications": Record<string, never>;
			"/(default)/profile": Record<string, never>;
			"/(default)/questions": Record<string, never>;
			"/(default)/question": Record<string, never>;
			"/(default)/roadmap": Record<string, never>;
			"/(default)/series": Record<string, never>;
			"/(default)/settings": Record<string, never>;
			"/(default)/skill-trees": Record<string, never>;
			"/(default)/skill-tree": Record<string, never>;
			"/(default)/skill-tree/new": Record<string, never>;
			"/(fullwidth)/skills": Record<string, never>;
			"/(default)/tag": Record<string, never>
		};
		Pathname(): "/" | "/about" | "/article" | "/book-edition" | "/books" | "/book" | "/drafts" | "/forks" | "/graph" | "/guide" | "/library" | "/new-question" | "/new-series" | "/new" | "/notifications" | "/profile" | "/questions" | "/question" | "/roadmap" | "/series" | "/settings" | "/skill-trees" | "/skill-tree" | "/skill-tree/new" | "/skills" | "/tag";
		ResolvedPathname(): `${"" | `/${string}`}${ReturnType<AppTypes['Pathname']>}`;
		Asset(): "/pytutor/codemirror-python.js" | "/pytutor/codemirror.js" | "/pytutor/coding.js" | "/pytutor/d3.v2.min.js" | "/pytutor/jquery-ui.min.js" | "/pytutor/jquery.ba-bbq.min.js" | "/pytutor/jquery.jsPlumb.min.js" | "/pytutor/jquery.min.js" | "/pytutor/pytutor.css" | "/pytutor/pytutor.js" | string & {};
	}
}