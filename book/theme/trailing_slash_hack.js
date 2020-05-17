// This is a very big hack to force a trailing slash in case the URL does not 
// point directly to an html document.
// 
// Netlify doesn't allow changing trailing slashes with redirects so we have to
// do it this way.
if (!window.location.pathname.endsWith(".html") &&
    !window.location.pathname.endsWith("/")) {
    var url = window.location.protocol + '//' + 
            window.location.host + 
            window.location.pathname + '/' + 
            window.location.search;

    if(window.history && window.history.replaceState) {
        window.history.replaceState(null, document.title, url);
    } else {
        window.location = url
    }
}
