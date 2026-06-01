// pdftract inspector UI application (stub)

(function() {
    'use strict';

    const viewer = document.getElementById('viewer');

    function init() {
        console.log('pdftract inspector UI initialized (stub)');
        // TODO: Load PDF data and render extraction overlays
        viewer.innerHTML = '<p class="placeholder">Inspector UI stub — awaiting Phase 7.9 implementation</p>';
    }

    // Initialize on DOM ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
