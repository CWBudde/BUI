// For the first-frame apps: once the first frame is on screen, print how long
// that took (ms since the runtime started) and the resident memory then, and quit.
'use strict'
module.exports = function report ({ app, window }) {
  window.requestAnimationFrame(() => {
    console.log(`first-frame ${performance.now().toFixed(1)} rss ${(process.memoryUsage.rss() / 1048576).toFixed(1)}`)
    app.quit()
  })
}
