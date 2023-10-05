<script lang="ts">
  import {
    Input,
    Container,
    Col,
    Row,
    Card,
    CardFooter,
    CardHeader,
    CardBody,
    CardSubtitle,
    Button,
  } from 'sveltestrap';
  import { onDestroy } from 'svelte';
  import prettyBytes from 'pretty-bytes';
  import { dataUrlToRawFile, rawFileToDataUrl } from './utils';
  import { pickingStripes } from './store';
  import type { ImageInfo, RgbColorReplacementPair } from './typeshare';
  import type { PickingStripeType, ProcessingImageType } from './types';
  import CanvasPicture from './CanvasPicture.svelte';
  import ImagePlaceholder from './ImagePlaceholder.svelte';
  import PageHeader from './PageHeader.svelte';
  import PickingStripeList from './PickingStripeList.svelte';

  export let bindings;

  let { image_info, replace_rgb_colors } = bindings; // destructure for easier access

  let original_file: File;
  let original_image: ProcessingImageType;
  let original_image_raw: Uint8Array;
  let processed_image: ProcessingImageType;
  let processing_indication = false;
  let image_caption: string =
    'This is a placeholder, please upload a .PNG file';

  let stripes: PickingStripeType[] = [];
  onDestroy(
    pickingStripes.subscribe((value: PickingStripeType[]) => {
      stripes = value;
    }),
  );

  let reader = new FileReader();
  reader.onload = (event) => {
    let image_content = event.target.result;
    original_image_raw = dataUrlToRawFile(image_content);

    const info: ImageInfo = image_info(original_image_raw);
    console.log('image info:', info);

    console.log(
      'image_info:',
      original_file.name,
      `${original_file.size} bytes`,
      info,
    );
    const { width, height, rgb_colors } = info;
    original_image = {
      width,
      height,
      content: rawFileToDataUrl(image_content),
    };
    // console.log("##2##", image_content)
    const { name, size } = original_file;
    image_caption = `${name} ${prettyBytes(
      size,
    )}, ${width} x ${height}, ${rgb_colors} colors`;
  };

  const onOriginalFileSelected = (event) => {
    original_file = event.target.files[0];
    original_image = undefined;
    processed_image = undefined;
    pickingStripes.set([]);
    // reader.readAsArrayBuffer(original_file);
    reader.readAsDataURL(original_file);
    console.log('original_file', typeof original_file, original_file);
  };

  const onUpdateButtonPressed = () => {
    console.log('It is going to process image');
    if (original_image_raw) {
      if (stripes.length) {
        processing_indication = true;
        setTimeout(() => {
          const pairs: RgbColorReplacementPair[] = stripes.map(
            ({ original, nominal }) => ({
              from: original,
              to: nominal,
            }),
          );
          // const raw = convert(original_image_raw, pairs);
          const result = replace_rgb_colors(original_image_raw, pairs);
          console.log('Image conversion is done');
          const { width, height } = original_image;
          processed_image = {
            width,
            height,
            content: rawFileToDataUrl(result.image),
          };
          // console.log(processed_image.content);
          processing_indication = false;
        }, 100);
      } else {
        processed_image = original_image;
      }
    }
  };

  // greet();
</script>

<Container>
  <Row>
    <Col>
      <PageHeader />
    </Col>
  </Row>

  <Row cols={2}>
    <Col>
      <Card>
        <CardHeader>
          <CardSubtitle>{@html image_caption}</CardSubtitle>
        </CardHeader>
        <CardBody>
          {#if original_image}
            <CanvasPicture image={original_image} />
          {:else if original_file}
            <h1>Loading...</h1>
          {:else}
            <ImagePlaceholder />
          {/if}
        </CardBody>
        <CardFooter>
          <Input
            type="file"
            accept=".jpg, .jpeg, .png"
            on:change={onOriginalFileSelected}
          />
        </CardFooter>
      </Card>
    </Col>
    <Col>
      <Card>
        <CardHeader>
          <CardSubtitle>
            Processed image
            {#if !processed_image}
              will be here
            {/if}
          </CardSubtitle>
        </CardHeader>
        <CardBody>
          {#if processing_indication}
            <h1>Processing...</h1>
          {:else if processed_image}
            <CanvasPicture image={processed_image} />
          {:else}
            <ul>
              <li>Upload image</li>
              <li>Select color pairs by clicking on pixel you like</li>
              <li>Press "Update" below</li>
            </ul>
          {/if}
        </CardBody>
        <CardFooter>
          <Button
            block
            color="light"
            disabled={!original_image}
            on:click={onUpdateButtonPressed}>Update</Button
          >
        </CardFooter>
      </Card>
    </Col>
  </Row>

  <PickingStripeList />
</Container>
