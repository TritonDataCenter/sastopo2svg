
//
// Populate the Host Information table when the document is loaded.
//
document.addEventListener("DOMContentLoaded", function() {
    var hostprops = document.getElementById("hostprops");
    
    var cell = document.getElementById("nodename");
    cell.innerHTML = hostprops.getAttribute("nodename");

    cell = document.getElementById("os-version");
    cell.innerHTML = hostprops.getAttribute("os-version");
    
    cell = document.getElementById("timestamp");
    cell.innerHTML = hostprops.getAttribute("timestamp");
});

//
// When a graph vertex is clicked in the SVG, highlight the clicked vertex and
// and populate the info panel on the left side with the properties of that
// vertex.
//
function showInfo(evt) {
    //
    // Iterate through the DOM <rect> elements, which represent the graph
    // vertices and set the fill color to none.
    //
    var allrects = document.getElementsByTagName("rect");
    for (var i = 0; i < allrects.length; i++) {
        allrects[i].setAttribute("fill", "none");
    }

    // Highlight the vertex that was clicked by setting the fill color
    var rect = evt.target.parentElement.getElementsByTagName("rect");
    rect[0].setAttribute("fill", "cyan");

    // Clear the Node Information table
    var nodeinfo = document.getElementById("nodeinfo");
    var numrows = nodeinfo.rows.length;
    for (var i = 0; i < numrows; i++) {
        console.log("deleting row ...");
        nodeinfo.deleteRow(-1);
    }

    var group = evt.target.parentElement;
    var props;
    var name = group.getAttribute("name");

    if (name === "initiator") {
        props = ["fmri", "hc-fmri", "devfs-name", "name", "manufacturer", "model", "serial", "label"];
    } else if (name === "port") {
        props = ["fmri", "name", "local-sas-address", "attached-sas-address"];
    } else if (name === "expander") {
        props = ["fmri", "name", "devfs-name"];
    } else if (name === "target") {
        props = ["fmri", "hc-fmri", "name", "manufacturer", "model", "serial", "label"];
    }

    for (const prop of props) {
        var value = group.getAttribute(prop);
        //
        // The value for hc-fmri can be quite long, so to make it fit better in
        // the info panel, we strip out the authority portion of the fmri.
        //
        if (prop === "hc-fmri") {
            end_auth = value.indexOf("/", 6);
            if (end_auth != -1) {
                value = "hc://" + value.substring(end_auth);
            }
        }
        var row = nodeinfo.insertRow(-1);
        var fieldcell = row.insertCell(-1);
        fieldcell.innerHTML = prop;
        var valuecell = row.insertCell(-1);
        valuecell.innerHTML = value;
    }
}
