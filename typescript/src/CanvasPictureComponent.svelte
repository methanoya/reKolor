<script lang="ts">
  import { htmlColor, rgba2rgb } from './utils';

  export let image: ProcessingImageType;

  import { Stage, Layer, Rect, Image } from 'svelte-konva';
  import { afterUpdate, onMount } from 'svelte';
  import _ from 'lodash';
  import { pickingStripes } from './store';
  import type { ProcessingImageType } from './types';
  import { pantone } from './palette';

  let clientWidth: number, clientHeight: number;
  let stageWidth: number, stageHeight: number;
  let imageFactor: number = 1;
  let htmlImage: HTMLImageElement;
  let rectColor: string = 'blue';

  onMount(() => {
    const imgTag = document.createElement('img');
    // img.src = "https://konvajs.org/assets/yoda.jpg";
    // console.log("!!! image:", image)
    imgTag.src = image.content;
    imgTag.onload = () => {
      htmlImage = imgTag;
      // console.log(
      //   '!!! onMount::load !!!',
      //   image.width,
      //   image.height,
      //   clientWidth,
      //   clientHeight,
      // );
      htmlImage.width = imageFactor * image.width;
      htmlImage.height = imageFactor * image.height;
    };
  });

  afterUpdate(() => {
    if (htmlImage && htmlImage.src != image.content) {
      htmlImage.src = image.content;
      // console.log(
      //   '!!! afterUpdate !!!',
      //   image.width,
      //   image.height,
      //   clientWidth,
      //   clientHeight,
      // );
    }
  });

  const onMouse = (event) => {
    // console.log(event);
    const {
      detail: { target, evt },
    } = event;
    const layer = target?.parent;
    const { layerX, layerY } = evt;
    const { data } = layer
      .getCanvas()
      .getContext()
      .getImageData(layerX, layerY, 1, 1, 'srgb');
    let [r, g, b, a] = data;
    a = (a ?? 255) / 255;
    const picked = {
      r,
      g,
      b,
      a,
    };
    const original = rgba2rgb(picked);
    const [nominal, nominalName] = pantone(original);
    console.log(
      'Picked',
      r,
      g,
      b,
      a,
      'at',
      layerX,
      layerY,
      'matched to',
      nominal,
    );
    rectColor = htmlColor({ r, g, b, a });
    pickingStripes.update((list) => {
      for (const entry of list) {
        if (_.isEqual(entry.original, original)) {
          return list;
        }
      }
      return [
        ...list,
        {
          picked,
          original,
          nominal,
          nominalName,
        },
      ];
    });
  };

  $: {
    stageWidth = Math.min(clientWidth, image?.width ?? Number.MAX_VALUE);
    imageFactor = stageWidth / image.width;
    stageHeight = imageFactor * (image?.height || 0);
  }
</script>

<div
  bind:clientWidth
  bind:clientHeight
  class="d-flex justify-content-md-center align-items-center"
>
  <!--  <Figure {caption}>-->
  <Stage config={{ width: stageWidth, height: stageHeight }}>
    {#if htmlImage}
      <Layer>
        <Image config={{ image: htmlImage }} on:click={onMouse} />
      </Layer>
      <!--        <Layer>-->
      <!--          <Rect-->
      <!--            config={{-->
      <!--              x: 100,-->
      <!--              y: 100,-->
      <!--              width: 200,-->
      <!--              height: 200,-->
      <!--              fill: rectColor,-->
      <!--            }}-->
      <!--          />-->
      <!--        </Layer>-->
    {/if}
  </Stage>
  <!--  </Figure>-->
</div>
