<script lang="ts">
  import { Info, TrophyIcon } from '@lucide/svelte';
  import CollapsableModal from '../components/CollapsableModal.svelte';
  import { state } from '../state.svelte';
</script>

<CollapsableModal
	collapsedIcon={Info}
	isOpen={state.isInfoOpen}
	onOpen={() => {
		state.isSearchOpen = false;
		state.isInfoOpen = true;
	}}
	onClose={() => {
		state.isInfoOpen = false;
	}}
>
	<h2>All The Views In The World</h2>
	<p>We've calculated all the views on the planet.</p>
	<p>Click on any point to show the longest line of sight at that location</p>
	<p>
		The lines are the theoretical ideals. They rely on perfect weather
		conditions and favourable refraction.
	</p>
	<p>
		Heatmap colours: the brighter the more and further you can see. The darker
		the less you can see.
	</p>

	<p>
		Click the <span class="unclickable_icon"><TrophyIcon /></span> icon below to
		see our curated list of the planet's longest lines of sight
	</p>

	<p>
		See <a href="https://alltheviews.world">alltheviews.world</a> for more details.
	</p>
</CollapsableModal>
