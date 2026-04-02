'use strict'

/**
 * No-op migration shim. Nunjucks needs `installJinjaCompat()` for slice syntax;
 * Runjucks accepts Jinja-style slices without calling this. Import so legacy
 * `require('…/install-jinja-compat')` does not throw when porting apps.
 */
function installJinjaCompat() {}

module.exports = { installJinjaCompat }
