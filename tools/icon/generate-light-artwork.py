#!/usr/bin/env python3
"""Derive the light icon and transparent mark from the original rider artwork."""
import copy
from pathlib import Path
import sys
import xml.etree.ElementTree as ET

namespace = 'http://www.w3.org/2000/svg'
ET.register_namespace('', namespace)
tag = lambda name: f'{{{namespace}}}{name}'
source = Path(__file__).with_name('icon.svg').read_text()
source = source.replace('#121418', '#f1f5f9').replace('#2dd4bf', '#14b8a6')
svg = ET.fromstring(source)
svg.set('viewBox', '-37.5 0 125 125')
svg.set('width', '1024')
svg.set('height', '1024')
rider = next(group for group in svg.findall(tag('g')) if group.get('id') == 'g4')

# Draw the contrasting edge underneath the original teal silhouette. Its
# existing one-unit stroke stays intact, with another 0.6 units outside it.
outline = copy.deepcopy(rider)
for element in outline.iter():
    for attribute in ('style', 'fill', 'stroke', 'id'):
        element.attrib.pop(attribute, None)
outline.set('fill', '#073832')
outline.set('stroke', '#073832')
outline.set('stroke-width', '2.2')
outline.set('stroke-linejoin', 'round')
artwork = ET.Element(tag('g'), {'filter': 'url(#rider-shadow)'})
svg.remove(rider)
artwork.extend([outline, rider])
svg.append(artwork)

shadow = ET.SubElement(svg.find(tag('defs')), tag('filter'), {
    'id': 'rider-shadow', 'x': '-20%', 'y': '-20%',
    'width': '140%', 'height': '145%', 'color-interpolation-filters': 'sRGB',
})
ET.SubElement(shadow, tag('feGaussianBlur'), {
    'in': 'SourceAlpha', 'stdDeviation': '0.9', 'result': 'blur',
})
ET.SubElement(shadow, tag('feOffset'), {
    'in': 'blur', 'dx': '0', 'dy': '1.1', 'result': 'offset',
})
ET.SubElement(shadow, tag('feFlood'), {
    'flood-color': '#052e2b', 'flood-opacity': '0.26', 'result': 'tint',
})
ET.SubElement(shadow, tag('feComposite'), {
    'in': 'tint', 'in2': 'offset', 'operator': 'in', 'result': 'drop',
})
merge = ET.SubElement(shadow, tag('feMerge'))
ET.SubElement(merge, tag('feMergeNode'), {'in': 'drop'})
ET.SubElement(merge, tag('feMergeNode'), {'in': 'SourceGraphic'})

destination = Path(sys.argv[1])
ET.ElementTree(svg).write(destination / 'icon-light.svg', encoding='unicode')
svg.remove(svg.find(tag('rect')))
ET.ElementTree(svg).write(destination / 'icon-light-mark.svg', encoding='unicode')
