//
// When a graph vertex is clicked in the SVG, highlight the clicked vertex and
// and populate the info panel on the left side with the properties of that
// vertex.
//
function showInfo(evt) {
    var infobox;

    //
    // Iterate through the DOM <rect> elements, which represent the graph
    // vertices and set the fill color to none.  While we're iterating,
    // cache a reference to the group element for the info panel, which
    // we'll need later.
    //
    var allrects = document.getElementsByTagName("rect");
    for (var i = 0; i < allrects.length; i++) {
        allrects[i].setAttribute("fill", "none");
        var name = allrects[i].getAttribute("name");
        if (name === "infobox") {
            infobox = allrects[i].parentElement;
        }
    }

    // Highlight the vertex that was clicked by setting the fill color
    var rect = evt.target.parentElement.getElementsByTagName("rect");
    rect[0].setAttribute("fill", "cyan");

    //
    // Clear the info panel by iterating through the child <text? elements of
    // the info panel and removing the ones that have an a special attribute
    // that indicates there are a vertex property.
    //
    var texts = infobox.getElementsByTagName("text");
    var textarr = Array.from(texts);
    for (var i = 0; i < textarr.length; i++) {
        var id = textarr[i].getAttribute("id");
        if (id === "nodeproperty") {
            infobox.removeChild(textarr[i]);
        }
    }

    //
    // Finally, create new <text> elements for the properties of the vertex
    // that was clicked on.
    //
    var prop_x = 15;
    var prop_y = 150;
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
        var prop_element = document.createElementNS("http://www.w3.org/2000/svg", "text");
        prop_element.setAttribute("x", prop_x);
        prop_element.setAttribute("y", prop_y);
        prop_element.style.fontFamily = "Courier New, Courier, monospace";
        prop_element.setAttribute("id", "nodeproperty");
        prop_element.innerHTML = prop + ": " + value;
        infobox.appendChild(prop_element);
        prop_y = prop_y + 20;
    }
}
