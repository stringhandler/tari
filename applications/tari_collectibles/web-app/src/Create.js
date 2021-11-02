//  Copyright 2021. The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
import React, { useState, useMemo } from "react";
import {
  Alert,
  Button,
  Container,
  FormControlLabel,
  FormGroup,
  List,
  ListItem,
  ListItemText,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@mui/material";
import binding from "./binding";
import { withRouter } from "react-router-dom";
import { appWindow } from "@tauri-apps/api/window";
import { dialog } from "@tauri-apps/api";
import { Command } from "@tauri-apps/api/shell";
import { fetch, ResponseType } from "@tauri-apps/api/http";

class Create extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      name: "Asset1",
      description: "",
      image: "",
      cid: "",
      error: "",
      isSaving: false,
      contract001: true,
      contract721: false,
      contract100: false,
      numberInitialTokens: 0,
      committee: [],
      newCommitteePubKey: "",
    };

    this.cleanup = null;
  }

  componentDidMount() {
    this.cleanup = appWindow.listen("tauri://file-drop", this.dropFile);
  }

  dropFile = async ({ payload }) => {
    if (payload.length > 0) {
      // only use the first file if multiple are dropped
      this.addFileToIPFS(payload[0]);
    }
  };

  componentWillUnmount() {
    if (this.cleanup) {
      this.cleanup.then((unlisten) => unlisten());
    }
  }

  save = async () => {
    this.setState({ isSaving: true });
    let name = this.state.name;
    let description = this.state.description;
    let image = this.state.image;
    try {
      let publicKey = await binding.command_assets_create(
        name,
        description,
        image
      );

      if (this.state.contract721 || this.state.contract100) {
        let res = await binding.command_asset_issue_simple_tokens(
          publicKey,
          parseInt(this.state.numberInitialTokens),
          this.state.committee
        );

        console.log(res);
      }
      let history = this.props.history;

      history.push(`/assets/manage/${publicKey}`);
    } catch (err) {
      this.setState({
        error: "Could not create asset: " + err,
      });
      console.log(err);
    }
    this.setState({ isSaving: false });
  };

  onNameChanged = (e) => {
    this.setState({ name: e.target.value });
  };

  onContract001Changed = (e) => {
    this.setState({ contract001: e.target.checked });
  };

  onContract721Changed = (e) => {
    this.setState({ contract721: e.target.checked });
  };

  onContract100Changed = (e) => {
    this.setState({ contract100: e.target.checked });
  };

  onNumberInitialTokensChanged = (e) => {
    this.setState({ numberInitialTokens: e.target.value });
  };

  onDescriptionChanged = (e) => {
    this.setState({
      description: e.target.value,
    });
  };

  onNewCommitteePubKeyChanged = (e) => {
    this.setState({
      newCommitteePubKey: e.target.value,
    });
  };

  onAddCommitteeMember = () => {
    let committee = [...this.state.committee];
    committee.push(this.state.newCommitteePubKey);
    console.log(committee);
    this.setState({
      committee,
      newCommitteePubKey: "",
    });
  };

  onDeleteCommitteeMember = (index) => {
    let committee = this.state.committee.filter(function (_, i, arr) {
      return i !== parseInt(index);
    });

    this.setState({ committee });
  };

  onImageChanged = (e) => {
    this.setState({
      image: e.target.value,
    });
  };

  selectFile = async () => {
    const filePath = await dialog.open({
      filters: [
        { name: "image types", extensions: ["png", "jpg", "jpeg", "gif"] }, // TODO more formats
      ],
      multiple: false,
    });
    await this.addFileToIPFS(filePath);
  };

  addFileToIPFS = async (filePath) => {
    const parts = filePath.split("/");
    const name = parts[parts.length - 1];
    // unfortunately the ipfs http /add api doesn't play nicely with the tauri http client
    // resulting in "file argument 'path' is required"
    // so we'll shell out to the ipfs command
    let cid = "";
    try {
      const command = new Command("ipfs", ["add", "-Q", filePath]);
      let processing = false;
      command.stdout.on("data", (line) => {
        if (!processing) {
          processing = true;
          cid = line;
          const cp = new Command("ipfs", [
            "files",
            "cp",
            `/ipfs/${cid}`,
            `/${name}`,
          ]);
          cp.spawn();
          this.setState({ cid });
        }
      });
      command.stderr.on("data", (line) => {
        console.error("error: ", line);
      });
      const child = command.spawn();
      return child;

      // console.log("command", command);
      // const { success, output, error } = commandOutput(command);
      // if (success) {
      //   const cid = output;
      //   console.log("cid", cid);
      //   const command = await runCommand("ipfs", [
      //     "files",
      //     "cp",
      //     `/ipfs/${cid}`,
      //     `/${name}`,
      //   ]);
      //   const { error } = commandOutput(command);
      //   if (error) console.error("error: ", error);
      // } else {
      //   console.error("error: ", error);
      // }
    } catch (e) {
      // todo: display error
      console.error("catch error: ", e);
    }
  };

  render() {
    return (
      <Container maxWidth="lg" sx={{ mt: 4, mb: 4, py: 8 }}>
        <Typography variant="h3" sx={{ mb: "30px" }}>
          Create new asset
        </Typography>
        <Stack>
          {this.state.error ? (
            <Alert severity="error">{this.state.error}</Alert>
          ) : (
            <span />
          )}
          <Typography variant="h4">Templates Implemented</Typography>
          <FormControlLabel
            control={
              <Switch
                onChange={this.onContract001Changed}
                checked={this.state.contract001}
              />
            }
            label="001 Metadata (required)"
          />

          <FormGroup>
            <TextField
              id="name"
              label="Name"
              variant="filled"
              color="primary"
              value={this.state.name}
              onChange={this.onNameChanged}
              disabled={this.state.isSaving || !this.state.contract001}
            ></TextField>
            <TextField
              id="description"
              label="Description"
              variant="filled"
              color="primary"
              value={this.state.description}
              onChange={this.onDescriptionChanged}
              disabled={this.state.isSaving || !this.state.contract001}
            ></TextField>

            <p>Image</p>
            <ImageSelector
              selectFile={this.selectFile}
              setImage={(image) => this.setState({ image })}
              cid={this.state.cid}
              setCid={(cid) => this.setState({ cid })}
              image={this.state.image}
            />

            <FormControlLabel
              control={
                <Switch
                  onClick={this.onContract721Changed}
                  checked={this.state.contract721}
                />
              }
              label="721 (ERC 721-like)"
            />
            <TextField
              id="numTokens"
              value={this.state.numberInitialTokens}
              onChange={this.onNumberInitialTokensChanged}
              type="number"
              label="Number of initial tokens"
              disabled={this.state.isSaving || !this.state.contract721}
            ></TextField>
          </FormGroup>
          <FormGroup>
            <FormControlLabel
              control={
                <Switch
                  onClick={this.onContract100Changed}
                  checked={this.state.contract100}
                />
              }
              label="100 Sidechain with committees"
            />
          </FormGroup>
          <FormGroup>
            <List>
              {this.state.committee.map((item, index) => {
                return (
                  <ListItem>
                    <ListItemText primary={item}></ListItemText>
                  </ListItem>
                );
              })}
            </List>
            <TextField
              label="Validator node public key"
              id="newCommitteePubKey"
              value={this.state.newCommitteePubKey}
              onChange={this.onNewCommitteePubKeyChanged}
              disabled={this.state.isSaving || !this.state.contract100}
            ></TextField>
            <Button
              onClick={this.onAddCommitteeMember}
              disabled={this.state.isSaving || !this.state.contract100}
            >
              Add
            </Button>
          </FormGroup>

          <Button onClick={this.save} disabled={this.state.isSaving}>
            Save
          </Button>
        </Stack>
      </Container>
    );
  }
}

const ImageSwitch = ({ setMode }) => {
  return (
    <div>
      <Button onClick={() => setMode("url")}>HTTP or IPFS URL</Button>
      <Button onClick={() => setMode("upload")}>Upload file to IPFS</Button>
    </div>
  );
};

const ImageUrl = ({ setImage }) => {
  const [url, setUrl] = useState("");

  return (
    <div>
      <p>Link to a web or ipfs url</p>
      <TextField
        id="image"
        label="Image (url)"
        variant="filled"
        color="primary"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
      ></TextField>
      <Button onClick={() => setImage(url)}>Save</Button>
    </div>
  );
};

const ImageUpload = ({ selectFile }) => {
  return (
    <div>
      <p>Select an image, or drag and drop an image onto this window</p>
      <Button onClick={selectFile}>Click to Select Image</Button>
    </div>
  );
};

const ImageSelector = ({ cid, image, selectFile, setImage, setCid }) => {
  const [mode, setMode] = useState("");

  if (image) {
    return (
      <div>
        <img src={image} alt="" width="200px" />
        <br />
        <p onClick={() => setImage("")}>Change</p>
      </div>
    );
  }
  if (cid) {
    return <IpfsImage cid={cid} setCid={setCid} />;
  }

  let display;

  switch (mode) {
    case "url":
      display = <ImageUrl setImage={setImage} />;
      break;
    case "upload":
      display = <ImageUpload selectFile={selectFile} />;
      break;
    default:
      display = <ImageSwitch setMode={(m) => setMode(m)} />;
  }

  return display;
};

const IpfsImage = ({ cid, setCid }) => {
  const [src, setSrc] = useState("");

  useMemo(async () => {
    try {
      const response = await fetch(
        `http://localhost:5001/api/v0/cat?arg=${cid}`,
        {
          method: "POST",
          responseType: ResponseType.Binary,
        }
      );
      const typedArray = Uint8Array.from(response.data);
      const blob = new Blob([typedArray.buffer]);
      const reader = new FileReader();
      reader.onloadend = () => setSrc(reader.result);
      reader.onerror = () => console.error("error");
      reader.readAsDataURL(blob);
    } catch (e) {
      // handle error
      console.error(e);
    }
  }, [cid]);

  if (src) {
    return (
      <div>
        <img src={src} alt="" width="200px" />
        <br />
        <p onClick={() => setCid("")}>Change</p>
      </div>
    );
  }

  return <p>ipfs image loading...</p>;
};

export default withRouter(Create);