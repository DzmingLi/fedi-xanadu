export const manifest = (() => {
function __memo(fn) {
	let value;
	return () => value ??= (value = fn());
}

return {
	appDir: "_app",
	appPath: "_app",
	assets: new Set(["pytutor/codemirror-python.js","pytutor/codemirror.js","pytutor/coding.js","pytutor/d3.v2.min.js","pytutor/jquery-ui.min.js","pytutor/jquery.ba-bbq.min.js","pytutor/jquery.jsPlumb.min.js","pytutor/jquery.min.js","pytutor/pytutor.css","pytutor/pytutor.js"]),
	mimeTypes: {".js":"text/javascript",".css":"text/css"},
	_: {
		client: {start:"_app/immutable/entry/start.DNCDcwzE.js",app:"_app/immutable/entry/app.DpZHUmCR.js",imports:["_app/immutable/entry/start.DNCDcwzE.js","_app/immutable/chunks/CQVI7Ke7.js","_app/immutable/chunks/Dwit3O6e.js","_app/immutable/chunks/fHOW8_U5.js","_app/immutable/chunks/kslRJjCZ.js","_app/immutable/entry/app.DpZHUmCR.js","_app/immutable/chunks/Dp1pzeXC.js","_app/immutable/chunks/Dwit3O6e.js","_app/immutable/chunks/CZ77qDrY.js","_app/immutable/chunks/CScAu-Jo.js","_app/immutable/chunks/kslRJjCZ.js","_app/immutable/chunks/BMevTLlB.js","_app/immutable/chunks/B864LTPl.js","_app/immutable/chunks/BqEPHt_d.js","_app/immutable/chunks/Cz-_V-MG.js","_app/immutable/chunks/BREhjVXi.js","_app/immutable/chunks/fHOW8_U5.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
		nodes: [
			__memo(() => import('./nodes/0.js')),
			__memo(() => import('./nodes/1.js')),
			__memo(() => import('./nodes/2.js')),
			__memo(() => import('./nodes/3.js')),
			__memo(() => import('./nodes/4.js')),
			__memo(() => import('./nodes/5.js')),
			__memo(() => import('./nodes/6.js')),
			__memo(() => import('./nodes/7.js')),
			__memo(() => import('./nodes/8.js')),
			__memo(() => import('./nodes/9.js')),
			__memo(() => import('./nodes/10.js')),
			__memo(() => import('./nodes/11.js')),
			__memo(() => import('./nodes/12.js')),
			__memo(() => import('./nodes/13.js')),
			__memo(() => import('./nodes/14.js')),
			__memo(() => import('./nodes/15.js')),
			__memo(() => import('./nodes/16.js')),
			__memo(() => import('./nodes/17.js')),
			__memo(() => import('./nodes/18.js')),
			__memo(() => import('./nodes/19.js')),
			__memo(() => import('./nodes/20.js')),
			__memo(() => import('./nodes/21.js')),
			__memo(() => import('./nodes/22.js')),
			__memo(() => import('./nodes/23.js')),
			__memo(() => import('./nodes/24.js')),
			__memo(() => import('./nodes/25.js')),
			__memo(() => import('./nodes/26.js')),
			__memo(() => import('./nodes/27.js')),
			__memo(() => import('./nodes/28.js')),
			__memo(() => import('./nodes/29.js')),
			__memo(() => import('./nodes/30.js')),
			__memo(() => import('./nodes/31.js')),
			__memo(() => import('./nodes/32.js'))
		],
		remotes: {
			
		},
		routes: [
			{
				id: "/(home)",
				pattern: /^\/?$/,
				params: [],
				page: { layouts: [0,5,], errors: [1,,], leaf: 31 },
				endpoint: null
			},
			{
				id: "/(default)/about",
				pattern: /^\/about\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 8 },
				endpoint: null
			},
			{
				id: "/(article)/article",
				pattern: /^\/article\/?$/,
				params: [],
				page: { layouts: [0,2,], errors: [1,,], leaf: 7 },
				endpoint: null
			},
			{
				id: "/(default)/book-edition",
				pattern: /^\/book-edition\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 9 },
				endpoint: null
			},
			{
				id: "/(default)/books",
				pattern: /^\/books\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 10 },
				endpoint: null
			},
			{
				id: "/(wide)/book",
				pattern: /^\/book\/?$/,
				params: [],
				page: { layouts: [0,6,], errors: [1,,], leaf: 32 },
				endpoint: null
			},
			{
				id: "/(default)/drafts",
				pattern: /^\/drafts\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 11 },
				endpoint: null
			},
			{
				id: "/(default)/forks",
				pattern: /^\/forks\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 12 },
				endpoint: null
			},
			{
				id: "/(fullwidth)/graph",
				pattern: /^\/graph\/?$/,
				params: [],
				page: { layouts: [0,4,], errors: [1,,], leaf: 28 },
				endpoint: null
			},
			{
				id: "/(default)/guide",
				pattern: /^\/guide\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 13 },
				endpoint: null
			},
			{
				id: "/(fullwidth)/library",
				pattern: /^\/library\/?$/,
				params: [],
				page: { layouts: [0,4,], errors: [1,,], leaf: 29 },
				endpoint: null
			},
			{
				id: "/(default)/new-question",
				pattern: /^\/new-question\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 15 },
				endpoint: null
			},
			{
				id: "/(default)/new-series",
				pattern: /^\/new-series\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 16 },
				endpoint: null
			},
			{
				id: "/(default)/new",
				pattern: /^\/new\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 14 },
				endpoint: null
			},
			{
				id: "/(default)/notifications",
				pattern: /^\/notifications\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 17 },
				endpoint: null
			},
			{
				id: "/(default)/profile",
				pattern: /^\/profile\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 18 },
				endpoint: null
			},
			{
				id: "/(default)/questions",
				pattern: /^\/questions\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 20 },
				endpoint: null
			},
			{
				id: "/(default)/question",
				pattern: /^\/question\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 19 },
				endpoint: null
			},
			{
				id: "/(default)/roadmap",
				pattern: /^\/roadmap\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 21 },
				endpoint: null
			},
			{
				id: "/(default)/series",
				pattern: /^\/series\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 22 },
				endpoint: null
			},
			{
				id: "/(default)/settings",
				pattern: /^\/settings\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 23 },
				endpoint: null
			},
			{
				id: "/(default)/skill-trees",
				pattern: /^\/skill-trees\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 26 },
				endpoint: null
			},
			{
				id: "/(default)/skill-tree",
				pattern: /^\/skill-tree\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 24 },
				endpoint: null
			},
			{
				id: "/(default)/skill-tree/new",
				pattern: /^\/skill-tree\/new\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 25 },
				endpoint: null
			},
			{
				id: "/(fullwidth)/skills",
				pattern: /^\/skills\/?$/,
				params: [],
				page: { layouts: [0,4,], errors: [1,,], leaf: 30 },
				endpoint: null
			},
			{
				id: "/(default)/tag",
				pattern: /^\/tag\/?$/,
				params: [],
				page: { layouts: [0,3,], errors: [1,,], leaf: 27 },
				endpoint: null
			}
		],
		prerendered_routes: new Set([]),
		matchers: async () => {
			
			return {  };
		},
		server_assets: {}
	}
}
})();
