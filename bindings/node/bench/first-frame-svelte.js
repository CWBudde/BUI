// bench/first-frame.js in Svelte 5 (bench/FirstFrame.svelte). Used by
// `bench/startup.js --svelte`.
'use strict'
const { render } = require('@blender-ui/svelte')
const App = require('./FirstFrame.svelte').default

require('./first-frame-report')(render(App, { title: 'first frame', width: 360, height: 200, gpu: true }))
