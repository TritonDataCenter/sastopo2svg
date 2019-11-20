
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
    // vertices and set the fill color to white.
    //
    var allrects = document.getElementsByTagName("rect");
    for (var i = 0; i < allrects.length; i++) {
        allrects[i].setAttribute("fill", "white");
    }

    // Highlight the vertex that was clicked by setting the fill color
    var rect = evt.target.parentElement.getElementsByTagName("rect");
    rect[0].setAttribute("fill", "cyan");

    // Clear the Node Information table
    var nodeinfo = document.getElementById("nodeinfo");
    var numrows = nodeinfo.rows.length;
    for (var i = 0; i < numrows; i++) {
        nodeinfo.deleteRow(-1);
    }

    // Clear and hide the PHY err table
    var errtable = document.getElementById("errtable");
    errtable.hidden = true;
    var errinfo = document.getElementById("errinfo");
    numrows = errinfo.rows.length;
    for (var i = 0; i < numrows; i++) {
        errinfo.deleteRow(-1);
    }

    var group = evt.target.parentElement;
    var link_err_props = ["invalid-dword", "running-disparity-error",
        "loss-dword-sync", "reset-problem-count"];
    var props;
    var name = group.getAttribute("name");

    if (name === "initiator") {
        props = ["fmri", "hc-fmri", "devfs-name", "name", "manufacturer",
            "model", "serial", "label"];
    } else if (name === "port") {
        props = ["fmri", "name", "sas-port-type", "local-sas-address",
            "attached-sas-address"];
    } else if (name === "expander") {
        props = ["fmri", "name", "devfs-name"];
    } else if (name === "target") {
        props = ["fmri", "hc-fmri", "name", "logical-disk", "manufacturer",
            "model", "serial-number", "label"];
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
        fieldcell.innerHTML = prop.bold();
        var valuecell = row.insertCell(-1);
        valuecell.colSpan = 4;
        valuecell.innerHTML = value;
    }

    if (name == "port") {
        //
        // Determine the number of phys on this port by scraping out the
        // start_phy and end_phy fields from the authority portion of this
        // node's sas-scheme FMRI.
        //
        var fmri = group.getAttribute("fmri");
        var regex = /start-phy=(\d+):end-phy=(\d+)/g;
        var match = regex.exec(fmri);
        var start_phy = match[1];
        var num_phys = (match[2] - match[1]) + 1;

        var phys = [];
        for (i = 0; i < num_phys; i++) {
            phys[i] = {
                [link_err_props[0]]: [],
                [link_err_props[1]]: [],
                [link_err_props[2]]: [],
                [link_err_props[3]]: []
            };
        }
        for (const prop of link_err_props) {
            var value = group.getAttribute(prop);
            if (value === undefined || value == null) {
                return;
            }
            value = value.split(",");
            for (phy = 0; phy < value.length; phy++) {
                phys[phy][prop].push(value[phy]);
            }
        }

        // Unhide the PHY Error table
        errtable.hidden = false;

        var hdrrow = errinfo.insertRow(-1);
        var hdrcell = hdrrow.insertCell(-1);
        hdrcell.innerHTML = "PHY #".bold();
        for (const prop of link_err_props) {
            hdrcell = hdrrow.insertCell(-1);
            hdrcell.innerHTML = prop.bold();
        }
        for (i = 0; i < num_phys; i++) {
            var errrow = errinfo.insertRow(-1);
            var errcell = errrow.insertCell(-1)
            errcell.innerHTML = (+start_phy + +i);
            for (const prop of link_err_props) {
                errcell = errrow.insertCell(-1);
                errcell.innerHTML = phys[i][prop];
            }
        }
    }
}
