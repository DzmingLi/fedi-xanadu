export { matchers } from './matchers.js';

export const nodes = [
	() => import('./nodes/0'),
	() => import('./nodes/1'),
	() => import('./nodes/2'),
	() => import('./nodes/3'),
	() => import('./nodes/4'),
	() => import('./nodes/5'),
	() => import('./nodes/6'),
	() => import('./nodes/7'),
	() => import('./nodes/8'),
	() => import('./nodes/9'),
	() => import('./nodes/10'),
	() => import('./nodes/11'),
	() => import('./nodes/12'),
	() => import('./nodes/13'),
	() => import('./nodes/14'),
	() => import('./nodes/15'),
	() => import('./nodes/16'),
	() => import('./nodes/17'),
	() => import('./nodes/18'),
	() => import('./nodes/19'),
	() => import('./nodes/20'),
	() => import('./nodes/21'),
	() => import('./nodes/22'),
	() => import('./nodes/23'),
	() => import('./nodes/24'),
	() => import('./nodes/25'),
	() => import('./nodes/26'),
	() => import('./nodes/27'),
	() => import('./nodes/28'),
	() => import('./nodes/29'),
	() => import('./nodes/30'),
	() => import('./nodes/31'),
	() => import('./nodes/32')
];

export const server_loads = [];

export const dictionary = {
		"/(home)": [31,[5]],
		"/(default)/about": [8,[3]],
		"/(article)/article": [7,[2]],
		"/(default)/book-edition": [9,[3]],
		"/(default)/books": [10,[3]],
		"/(wide)/book": [32,[6]],
		"/(default)/drafts": [11,[3]],
		"/(default)/forks": [12,[3]],
		"/(fullwidth)/graph": [28,[4]],
		"/(default)/guide": [13,[3]],
		"/(fullwidth)/library": [29,[4]],
		"/(default)/new-question": [15,[3]],
		"/(default)/new-series": [16,[3]],
		"/(default)/new": [14,[3]],
		"/(default)/notifications": [17,[3]],
		"/(default)/profile": [18,[3]],
		"/(default)/questions": [20,[3]],
		"/(default)/question": [19,[3]],
		"/(default)/roadmap": [21,[3]],
		"/(default)/series": [22,[3]],
		"/(default)/settings": [23,[3]],
		"/(default)/skill-trees": [26,[3]],
		"/(default)/skill-tree": [24,[3]],
		"/(default)/skill-tree/new": [25,[3]],
		"/(fullwidth)/skills": [30,[4]],
		"/(default)/tag": [27,[3]]
	};

export const hooks = {
	handleError: (({ error }) => { console.error(error) }),
	
	reroute: (() => {}),
	transport: {}
};

export const decoders = Object.fromEntries(Object.entries(hooks.transport).map(([k, v]) => [k, v.decode]));
export const encoders = Object.fromEntries(Object.entries(hooks.transport).map(([k, v]) => [k, v.encode]));

export const hash = false;

export const decode = (type, value) => decoders[type](value);

export { default as root } from '../root.js';