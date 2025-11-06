<script lang="ts" >
  import { Route, Router } from 'svelte5-router';
  import Home from './Home.svelte';
  import Packing from './Packing.svelte';

  export const url = '';
</script>

<Router {url}>
  <div>
    <Route path="/"><Home /></Route>
    <Route path="/tile-packing-the-world"><Packing /></Route>
  </div>
</Router>
