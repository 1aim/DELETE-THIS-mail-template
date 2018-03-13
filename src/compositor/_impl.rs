use std::borrow::Cow;

use serde::Serialize;
use vec1::Vec1;

use core::utils::HeaderTryFrom;
use core::error::{Result, ResultExt};
use core::header::HeaderMap;
use headers::{From, To, Subject, Sender};
use headers::components::Unstructured;
use mail::{Mail, Builder};

use utils::SerializeOnly;
use resource::{
    EmbeddingWithCId, Attachment,
    with_resource_sidechanel
};
use builder_extension::BuilderExt;
use template::{
    BodyPart, TemplateEngine, MailParts
};

use super::mail_send_data::MailSendData;
use super::{CompositionBase, EnvelopData};

pub(crate) trait InnerCompositionBaseExt: CompositionBase {

    fn _compose_mail<D>(
        &self,
        send_data:
            MailSendData<<Self::TemplateEngine as TemplateEngine<Self::Context>>::TemplateId, D>
    ) -> Result<(Mail, EnvelopData)>
        where D: Serialize
    {
        let envelop = EnvelopData::from(&send_data);
        //compose display name => create Address with display name;
        let (core_headers, data, template_id) = self.process_mail_send_data(send_data)?;

        let MailParts { alternative_bodies, shared_embeddings, attachments }
            = self.use_template_engine(&*template_id, data)?;

        let mail = self.build_mail( alternative_bodies, shared_embeddings.into_iter(),
                                    attachments, core_headers )?;

        Ok((mail, envelop))
    }

    fn process_mail_send_data<'a, D>(
        &self,
        send_data:
            MailSendData<'a, <Self::TemplateEngine as TemplateEngine<Self::Context>>::TemplateId, D>
    ) -> Result<(
        HeaderMap,
        D,
        Cow<'a, <Self::TemplateEngine as TemplateEngine<Self::Context>>::TemplateId>
    )>
        where D: Serialize
    {
        let (sender, from_mailboxes, to_mailboxes, subject, template_id, data)
            = send_data.destruct();

        // The subject header field
        let subject = Unstructured::try_from( subject )?;

        // createing the header map
        let mut core_headers: HeaderMap = headers! {
            //NOTE: if we support multiple mailboxes in From we have to
            // ensure Sender is used _iff_ there is more than one from
            From: from_mailboxes,
            To: to_mailboxes,
            Subject: subject
        }?;

        // add sender header if needed
        if let Some(sender) = sender {
            core_headers.insert(Sender, sender)?;
        }

        Ok((core_headers, data, template_id))
    }

    fn use_template_engine<D>(
        &self,
        template_id: &<Self::TemplateEngine as TemplateEngine<Self::Context>>::TemplateId,
        //TODO change to &D?
        data: D
    ) -> Result<MailParts>
        where D: Serialize
    {
        let id_gen = Box::new(self.context().clone());
        let ( mut mail_parts, embeddings, attachments ) =
            with_resource_sidechanel(id_gen, || -> Result<_> {
                // we just want to make sure that the template engine does
                // really serialize the data, so we make it so that it can
                // only do so (if we pass in the data directly it could use
                // TypeID+Transmut or TraitObject+downcast to undo the generic
                // type erasure and then create the template in some other way
                // but this would break the whole Embedding/Attachment extraction )
                let sdata = SerializeOnly::new(data);
                self.template_engine()
                    .use_templates(self.context(), template_id, &sdata)
                    .chain_err(|| "failure in template engine")
            })?;

        mail_parts.attachments.extend(attachments);
        mail_parts.shared_embeddings.extend(embeddings);
        Ok(mail_parts)
    }



    /// uses the results of preprocessing data and templates, as well as a list of
    /// mail headers like `From`,`To`, etc. to create a new mail
    fn build_mail<EMB>(&self,
                       bodies: Vec1<BodyPart>,
                       embeddings: EMB,
                       attachments: Vec<Attachment>,
                       core_headers: HeaderMap
    ) -> Result<Mail>
        where EMB: Iterator<Item=EmbeddingWithCId> + ExactSizeIterator
    {
        let mail = match attachments.len() {
            0 => Builder::create_alternate_bodies_with_embeddings(
                bodies, embeddings, Some(core_headers) )?,
            _n => Builder::create_with_attachments(
                Builder::create_alternate_bodies_with_embeddings(bodies, embeddings, None )?,
                attachments,
                Some( core_headers )
            )?
        };
        Ok( mail )
    }
}


impl<COT: ?Sized> InnerCompositionBaseExt for COT where COT: CompositionBase {}