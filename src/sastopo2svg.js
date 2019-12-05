'use strict';

var chassis_images = [
    'Joyent-M12G5',
    'Joyent-S10G5'
];

//
// Populate the Host Information table when the document is loaded.
//
var front_image_cell;
var rear_image_cell;
var product_id;
document.addEventListener('DOMContentLoaded', function () {
    var hostprops = document.getElementById('hostprops');
    product_id = hostprops.getAttribute('product-id');

    var cell = document.getElementById('product-id');
    cell.innerHTML = product_id;

    cell = document.getElementById('nodename');
    cell.innerHTML = hostprops.getAttribute('nodename');

    cell = document.getElementById('os-version');
    cell.innerHTML = hostprops.getAttribute('os-version');

    cell = document.getElementById('timestamp');
    cell.innerHTML = hostprops.getAttribute('timestamp');

    if (chassis_images.includes(product_id)) {
        var hostinfo = document.getElementById('hostinfo');
        var imgrow = hostinfo.insertRow(-1);
        front_image_cell = imgrow.insertCell(-1);
        front_image_cell.colSpan = 2;
        let img_file = product_id + '-front.png';
        front_image_cell.innerHTML = '<center><img src=\'assets/' +
            product_id + '/' + img_file + '\'></img></center>';

        imgrow = hostinfo.insertRow(-1);
        rear_image_cell = imgrow.insertCell(-1);
        rear_image_cell.colSpan = 2;
        img_file = product_id + '-rear.png';
        rear_image_cell.innerHTML = '<center><img src=\'assets/' +
            product_id + '/' + img_file + '\'></img></center>';
    }
});

var link_rate_strings = [
    'Unknown',
    'Disabled',
    'Reset problem',
    'Enabled, spin hold',
    'SATA link rate still negotiating',
    'Reset in progress',
    'Unsupported device attached',
    '0x07, Reserved',
    '1.5 Gbits/s',
    '3.0 Gbits/s',
    '6.0 GBits/s',
    '12.0 GBits/s',
    '22.5 GBits/s'
];

//
// When a graph vertex is clicked in the SVG, highlight the clicked vertex and
// and populate the info panel on the left side with the properties of that
// vertex.
//
function showInfo(evt) { // eslint-disable-line no-unused-vars
    //
    // Iterate through the DOM <img> elements, which represent the graph
    // vertices and set the fill color to white.
    //
    var allimgs = document.getElementsByTagName('image');
    for (let i = 0; i < allimgs.length; i++) {
        allimgs[i].setAttribute('filter', 'none');
    }

    //
    // Highlight the vertex that was clicked by setting the a filter on the
    // associated image element.
    //
    var img = evt.target.parentElement.getElementsByTagName('image');
    img[0].setAttribute('filter', 'url(#linear)');

    // Clear the Node Information table
    var nodeinfo = document.getElementById('nodeinfo');
    var numrows = nodeinfo.rows.length;
    for (let i = 0; i < numrows; i++) {
        nodeinfo.deleteRow(-1);
    }

    // Clear and hide the PHY link rate table
    var ratetable = document.getElementById('ratetable');
    ratetable.hidden = true;
    var rateinfo = document.getElementById('rateinfo');
    numrows = rateinfo.rows.length;
    for (let i = 0; i < numrows; i++) {
        rateinfo.deleteRow(-1);
    }

    // Clear and hide the PHY err table
    var errtable = document.getElementById('errtable');
    errtable.hidden = true;
    var errinfo = document.getElementById('errinfo');
    numrows = errinfo.rows.length;
    for (let i = 0; i < numrows; i++) {
        errinfo.deleteRow(-1);
    }

    var group = evt.target.parentElement;
    var link_rate_props = ['max-link-rate', 'negotiated-link-rate'];
    var link_err_props = ['invalid-dword', 'running-disparity-error',
        'loss-dword-sync', 'reset-problem-count'];
    var props;
    var name = group.getAttribute('name');

    if (name === 'initiator') {
        props = ['fmri', 'hc-fmri', 'devfs-name', 'name', 'manufacturer',
            'model', 'location'];
    } else if (name === 'port') {
        props = ['fmri', 'name', 'sas-port-type', 'local-sas-address',
            'attached-sas-address'];
    } else if (name === 'expander') {
        props = ['fmri', 'name', 'devfs-name'];
    } else if (name === 'target') {
        props = ['fmri', 'hc-fmri', 'name', 'logical-disk', 'manufacturer',
            'model', 'serial-number', 'location'];
    }

    for (const prop of props) {
        let value = group.getAttribute(prop);
        //
        // The value for hc-fmri can be quite long, so to make it fit better in
        // the info panel, we strip out the authority portion of the fmri.
        //
        if (prop === 'hc-fmri') {
            let end_auth = value.indexOf('/', 6);
            if (end_auth !== -1) {
                value = 'hc://' + value.substring(end_auth);
            }
        }
        var row = nodeinfo.insertRow(-1);
        var fieldcell = row.insertCell(-1);
        fieldcell.innerHTML = prop.bold();
        var valuecell = row.insertCell(-1);
        valuecell.colSpan = 4;
        valuecell.innerHTML = value;
    }
    let location = group.getAttribute('location');
    if (location !== null) {
        location = location.replace(/ /g, '-');
        let img_file = product_id + '-' + location;
        front_image_cell.innerHTML = '<center><img src=\'assets/' +
            product_id + '/' + img_file + '-front.png\'></img></center>';
        rear_image_cell.innerHTML = '<center><img src=\'assets/' +
            product_id + '/' + img_file + '-rear.png\'></img></center>';
    } else {
        let img_file = product_id + '-front.png';
        front_image_cell.innerHTML = '<center><img src=\'assets/' +
            product_id + '/' + img_file + '\'></img></center>';
        img_file = product_id + '-rear.png';
        rear_image_cell.innerHTML = '<center><img src=\'assets/' +
            product_id + '/' + img_file + '\'></img></center>';
    }

    if (name === 'port') {
        //
        // Determine the number of phys on this port by scraping out the
        // start_phy and end_phy fields from the authority portion of this
        // node's sas-scheme FMRI.
        //
        var fmri = group.getAttribute('fmri');
        /* JSSTYLED */
        var regex = /start-phy=(\d+):end-phy=(\d+)/g;
        var match = regex.exec(fmri);
        var start_phy = match[1];
        var num_phys = (match[2] - match[1]) + 1;

        var phys = [];
        for (let i = 0; i < num_phys; i++) {
            phys[i] = {
                [link_rate_props[0]]: [],
                [link_rate_props[1]]: []
            };
        }
        for (const prop of link_rate_props) {
            let value = group.getAttribute(prop);
            if (value === undefined || value === null) {
                return;
            }
            value = value.split(',');
            for (let phy = 0; phy < value.length; phy++) {
                let value_str = link_rate_strings[+value[phy]];
                phys[phy][prop].push(value_str);
            }
        }

        // Unhide the PHY Link Rate table
        ratetable.hidden = false;

        var hdrrow = rateinfo.insertRow(-1);
        var hdrcell = hdrrow.insertCell(-1);
        hdrcell.innerHTML = 'PHY #'.bold();
        for (const prop of link_rate_props) {
            hdrcell = hdrrow.insertCell(-1);
            hdrcell.innerHTML = prop.bold();
        }
        for (let i = 0; i < num_phys; i++) {
            let raterow = rateinfo.insertRow(-1);
            let ratecell = raterow.insertCell(-1);
            ratecell.innerHTML = (+start_phy + +i);
            for (const prop of link_rate_props) {
                ratecell = raterow.insertCell(-1);
                ratecell.innerHTML = phys[i][prop];
            }
        }

        phys = [];
        for (let i = 0; i < num_phys; i++) {
            phys[i] = {
                [link_err_props[0]]: [],
                [link_err_props[1]]: [],
                [link_err_props[2]]: [],
                [link_err_props[3]]: []
            };
        }
        for (const prop of link_err_props) {
            var value = group.getAttribute(prop);
            if (value === undefined || value === null) {
                return;
            }
            value = value.split(',');
            for (let phy = 0; phy < value.length; phy++) {
                phys[phy][prop].push(value[phy]);
            }
        }

        // Unhide the PHY Error table
        errtable.hidden = false;

        hdrrow = errinfo.insertRow(-1);
        hdrcell = hdrrow.insertCell(-1);
        hdrcell.innerHTML = 'PHY #'.bold();
        for (const prop of link_err_props) {
            hdrcell = hdrrow.insertCell(-1);
            hdrcell.innerHTML = prop.bold();
        }
        for (let i = 0; i < num_phys; i++) {
            var errrow = errinfo.insertRow(-1);
            var errcell = errrow.insertCell(-1);
            errcell.innerHTML = (+start_phy + +i);
            for (const prop of link_err_props) {
                errcell = errrow.insertCell(-1);
                errcell.innerHTML = phys[i][prop];
            }
        }
    }
}
