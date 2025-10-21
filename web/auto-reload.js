// Auto-reload script for development
(function() {
    let lastModified = {};

    function checkForUpdates() {
        fetch('/static/app.js?timestamp=' + Date.now(), {method: 'HEAD'})
            .then(response => {
                const lastMod = response.headers.get('last-modified');
                if (lastModified.js && lastModified.js !== lastMod) {
                    console.log('JavaScript file changed, reloading...');
                    location.reload();
                }
                lastModified.js = lastMod;
            })
            .catch(() => {});

        fetch('/static/style.css?timestamp=' + Date.now(), {method: 'HEAD'})
            .then(response => {
                const lastMod = response.headers.get('last-modified');
                if (lastModified.css && lastModified.css !== lastMod) {
                    console.log('CSS file changed, reloading...');
                    location.reload();
                }
                lastModified.css = lastMod;
            })
            .catch(() => {});
    }

    // Check for updates every 5 seconds in development
    if (location.hostname === 'localhost' || location.hostname === '127.0.0.1') {
        setInterval(checkForUpdates, 5000);
        console.log('Auto-reload enabled for development');
    }
})();