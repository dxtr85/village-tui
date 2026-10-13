use dapp_lib::prelude::{extract_chop_data, CastData, ChopData};
use smol::channel::Receiver;

pub mod poledit;
// pub use poledit::PolicyEditor;
pub async fn handle_bcast(recv: Receiver<CastData>) {
    // TODO: for larger files we might collect all pages
    // for one or more Contents before we AppendMultipleContents
    // to Datastore. dapp_lib should be able to cover this.
    // User should not have to worry about this ordering.
    // eprintln!("In HANDLE_BCAST");
    loop {
        if let Ok(msg) = recv.recv().await {
            // eprintln!("In handle_bcast, {:?}", msg.len());
            match extract_chop_data(msg) {
                Ok(ChopData::LeafHashes {
                    chop_nr,
                    chop_count,
                    content_nr,
                    content_count,
                    page_nr,
                    page_count,
                    data,
                }) => {
                    eprintln!(
                        "Got LeafHashes, CHOP:{}/{}, C:{}/{}, PG:{}/{} DLen:{}",
                        chop_nr,
                        chop_count,
                        content_nr,
                        content_count,
                        page_nr,
                        page_count,
                        data.len()
                    );
                }
                Ok(ChopData::LeafData {
                    chop_nr,
                    content_nr,
                    page_nr,
                    data,
                }) => {
                    eprintln!(
                        "Got LeafData, CHOP:{}, C:{}, PG:{} DLen:{}",
                        chop_nr,
                        content_nr,
                        page_nr,
                        data.len()
                    );
                }
                Err(c_data) => {
                    eprintln!("Got a msg, {:?}", c_data.len());
                }
            }
            // msg.first_byte()
        } else {
            eprintln!("E no bcast ");
        }
    }
}
