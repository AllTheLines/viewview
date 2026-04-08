<script lang="ts">
  import { Heart } from '@lucide/svelte';
  import CollapsableModal from '../components/CollapsableModal.svelte';
</script>

<CollapsableModal collapsedIcon={Heart} isOpen={false}>
	<h2>Acknowledgments</h2>
	<p>
		This project was made by <a href="https://tombh.co.uk">Tom</a> and
		<a href="https://ryan.berge.rs/">Ryan</a>. Contact us at
		<a href="mailto:hello@alltheviews.world"> hello@alltheviews.world </a>.
	</p>

	<p>
		The raw data comes from NASA's
		<a href="https://www.earthdata.nasa.gov/data/instruments/srtm">SRTM</a>
		mission. We used the cleaned version by
		<a href="http://www.viewfinderpanoramas.org/">Jonathon de Ferranti</a>.
	</p>

	<p>
		The vector map data is hosted by Zsolt Ero's <a
			href="https://openfreemap.org/">openfreemap.org</a
		> project.
	</p>
</CollapsableModal>
