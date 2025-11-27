<script lang="ts">
  import {
    LngLat,
    Map as MapLibre,
    Popup,
    type StyleSpecification,
  } from 'maplibre-gl';
  import { onMount } from 'svelte';
  import Layout from './Layout.svelte';
  import { state } from './state.svelte.ts';
  import { EARTH_RADIUS, toDegrees, toRadians } from './utils.ts';

  onMount(() => {
    state.map = new MapLibre({
      container: 'map',
      style: {
        version: 8,
        sources: {
          osm: {
            type: 'raster',
            tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
            tileSize: 256,
            attribution: '© OpenStreetMap contributors',
          },
        },
        layers: [
          {
            id: 'osm-tiles',
            type: 'raster',
            source: 'osm',
          },
        ],
      } as StyleSpecification,
    });

    state.map?.on('load', async () => {
      let tilesJSONURL = 'tiles.csv';
      const params = new URLSearchParams(self.location.search);
      let source = params.get('tiles');
      if (source) {
        tilesJSONURL = source;
      }
      const result = await fetch(tilesJSONURL);
      if (!result.ok) {
        throw new Error(`Failed to fetch tiles.json: ${result.status}`);
      }

      const tilesCSV = await result.text();
      state.map?.addSource('my-geojson', {
        type: 'geojson',
        data: tilesCSVToGeoJSON(tilesCSV),
      });
      state.map?.addLayer({
        id: 'fills',
        type: 'fill',
        source: 'my-geojson',
        paint: {
          'fill-outline-color': '#ff1234',
          'fill-color': '#ff0000',
          'fill-opacity': 0.35,
        },
      });
      state.map?.on('click', 'fills', (event) => {
        if (!state.map) {
          return;
        }
        const props = event.features?.[0].properties;
        let message = JSON.stringify(props, null, 2);
        message = message
          .replace('[', '[\n    ')
          .replace(']', '\n  ]')
          .replace(',', ',\n    ');
        new Popup()
          .setLngLat(event.lngLat)
          .setHTML(`<pre>${message}</pre>`)
          .addTo(state.map);
      });
      state.map?.on('mouseenter', 'fills', () => {
        if (!state.map) {
          return;
        }
        state.map.getCanvas().style.cursor = 'pointer';
      });
      state.map?.on('mouseleave', 'fills', () => {
        if (!state.map) {
          return;
        }
        state.map.getCanvas().style.cursor = '';
      });
    });
  });

  function tilesCSVToGeoJSON(csv: string) {
    const lines = csv.split(/\n/);
    let features = [];
    for (const line of lines) {
      if (line === '') {
        continue;
      }
      const parts = line.split(/,/);
      const lng = parseFloat(parts[0]);
      const lat = parseFloat(parts[1]);
      const centre = new LngLat(lng, lat);
      const width = parseFloat(parts[2]);
      const circle = makeCircle(centre, width / 2.0);
      features.push(circle);
    }

    const geojson = {
      type: 'FeatureCollection',
      features,
    } as GeoJSON.GeoJSON;

    return geojson;
  }

  // Find the lon/lat coordinate of a point based on its distance and bearing from another
  // point.
  function haversineDestination(
    origin: LngLat,
    bearing: number,
    meters: number,
  ) {
    const center_lng = toRadians(origin.lng);
    const center_lat = toRadians(origin.lat);
    const bearing_rad = toRadians(bearing);

    const radians = meters / EARTH_RADIUS;

    let lat = Math.asin(
      Math.sin(center_lat) * Math.cos(radians) +
        Math.cos(center_lat) * Math.sin(radians) * Math.cos(bearing_rad),
    );
    let lng =
      Math.atan2(
        Math.sin(bearing_rad) * Math.sin(radians) * Math.cos(center_lat),
        Math.cos(radians) - Math.sin(center_lat) * Math.sin(lat),
      ) + center_lng;

    let destination = new LngLat(toDegrees(lng), toDegrees(lat));

    return destination;
  }

  // Make a polygon representing the tile in lon/lat coordinates.
  function makeCircle(centre: LngLat, radius: number) {
    const resolution = 360;
    let coordinates = [];

    for (let i = 0; i <= resolution; i++) {
      const angle = (i * 360.0) / resolution;
      const destination = haversineDestination(centre, angle, radius);
      coordinates.push(destination.toArray());
    }

    return {
      type: 'Feature',
      geometry: {
        type: 'Polygon',
        coordinates: [coordinates],
      },
      properties: {
        centre: centre.toArray(),
        width: radius * 2,
      },
    };
  }
</script>

<Layout></Layout>

<style>
</style>
